use std::collections::HashMap;

use crate::{
    data::game_data::GameData,
    forward_fuse::is_available,
    model::{
        demon::{Demon, DemonId, NaturalSkill, SkillAcquisition},
        player_context::PlayerContext,
        recipe::RecipeMeta,
        route::{FusionSubroute, Route},
        skill::{SkillCategory, SkillId},
    },
    route_replay,
};

const MAX_EXPANDED_STATES: u32 = 131_072;
const MAX_SKILL_ASSIGNMENTS: u32 = 10_000_000;
const MAX_ROUTE_COMBINATIONS: u32 = 250_000;

type SkillMask = u8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchRequest {
    pub target: DemonId,
    pub required_skills: Vec<SkillId>,
    pub max_fusion_depth: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchSolution {
    pub route: Route,
    pub demon: Demon,
    pub fusion_depth: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchSafetyLimit {
    ExpandedStates { maximum: u32 },
    SkillAssignments { maximum: u32 },
    RouteCombinations { maximum: u32 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SearchError {
    UnknownDemon(DemonId),
    UnknownSkill(SkillId),
    UnsupportedSkill(SkillId),
    TooManySkills { selected: usize, maximum: usize },
    UnavailableTarget(DemonId),
    SafetyLimitExceeded(SearchSafetyLimit),
}

pub fn search(
    game_data: &GameData,
    player_context: &PlayerContext,
    request: &SearchRequest,
) -> Result<Vec<SearchSolution>, SearchError> {
    let target = game_data
        .demons()
        .get(request.target)
        .ok_or(SearchError::UnknownDemon(request.target))?;
    if !is_available(target, player_context) {
        return Err(SearchError::UnavailableTarget(request.target));
    }
    let skills = SkillUniverse::new(game_data, &request.required_skills)?;
    let required_skills = skills.full_mask();
    let mut solver = Solver {
        game_data,
        player_context,
        skills: &skills,
        memo: HashMap::new(),
        expanded_states: 0,
        skill_assignments: 0,
        route_combinations: 0,
    };
    let mut solutions = Vec::new();

    for fusion_depth in 0..=request.max_fusion_depth {
        let routes = solver
            .solve_exact(request.target, required_skills, fusion_depth)
            .map_err(SearchError::SafetyLimitExceeded)?;
        for route in routes {
            let demon = route_replay::replay(game_data, player_context, request, &route)
                .expect("search generated an invalid route");
            solutions.push(SearchSolution {
                route,
                demon,
                fusion_depth,
            });
        }
    }

    Ok(solutions)
}

struct SkillUniverse {
    skills: Vec<SkillId>,
}

impl SkillUniverse {
    fn new(game_data: &GameData, required_skills: &[SkillId]) -> Result<Self, SearchError> {
        let mut skills = required_skills.to_vec();
        skills.sort_unstable();
        skills.dedup();
        if skills.len() > Demon::MAX_SKILLS {
            return Err(SearchError::TooManySkills {
                selected: skills.len(),
                maximum: Demon::MAX_SKILLS,
            });
        }
        for &skill_id in &skills {
            let skill = game_data
                .skills()
                .get(skill_id)
                .ok_or(SearchError::UnknownSkill(skill_id))?;
            if skill.category == SkillCategory::Innate
                || skill.flags.magatsuhi
                || skill.flags.item_only
            {
                return Err(SearchError::UnsupportedSkill(skill_id));
            }
        }
        Ok(Self { skills })
    }

    fn full_mask(&self) -> SkillMask {
        ((1_u16 << self.skills.len()) - 1) as SkillMask
    }

    fn bit(&self, skill: SkillId) -> SkillMask {
        self.skills
            .binary_search(&skill)
            .map_or(0, |index| 1 << index)
    }

    fn skill_ids(&self, mask: SkillMask) -> Vec<SkillId> {
        self.skills
            .iter()
            .enumerate()
            .filter_map(|(index, skill)| (mask & (1 << index) != 0).then_some(*skill))
            .collect()
    }
}

fn enumerate_skill_assignments(
    skills_to_inherit: SkillMask,
    material_count: usize,
) -> Vec<Vec<SkillMask>> {
    if material_count == 0 {
        return Vec::new();
    }

    let skill_bits = (0..SkillMask::BITS)
        .map(|index| 1_u8 << index)
        .filter(|bit| skills_to_inherit & bit != 0)
        .collect::<Vec<_>>();
    let assignment_count = material_count.pow(skill_bits.len() as u32);
    let mut assignments = Vec::with_capacity(assignment_count);

    for assignment in 0..assignment_count {
        let mut encoded = assignment;
        let mut material_skills = vec![0; material_count];
        for &skill in skill_bits.iter().rev() {
            material_skills[encoded % material_count] |= skill;
            encoded /= material_count;
        }
        assignments.push(material_skills);
    }
    assignments
}

fn advance_combination<T>(indices: &mut [usize], options: &[Vec<T>]) -> bool {
    let Some(position) = (0..indices.len())
        .rev()
        .find(|position| indices[*position] + 1 < options[*position].len())
    else {
        return false;
    };
    indices[position] += 1;
    indices[position + 1..].fill(0);
    true
}

fn fusion_route_for_combination(
    skills: &SkillUniverse,
    recipe: &RecipeMeta,
    material_skills: &[SkillMask],
    material_route_options: &[Vec<(u32, Route)>],
    indices: &[usize],
    exact_fusion_depth: u32,
) -> Option<Route> {
    let maximum_child_depth = material_route_options
        .iter()
        .zip(indices)
        .map(|(options, index)| options[*index].0)
        .max()
        .expect("fusion recipes must have materials");
    if maximum_child_depth + 1 != exact_fusion_depth {
        return None;
    }

    let materials = material_skills
        .iter()
        .copied()
        .zip(material_route_options.iter().zip(indices))
        .map(|(required, (options, index))| FusionSubroute {
            required_skills: skills.skill_ids(required),
            route: options[*index].1.clone(),
        })
        .collect();
    Some(Route::Fusion {
        recipe: recipe.clone(),
        materials,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct StateKey {
    demon: DemonId,
    required_skills: SkillMask,
    exact_fusion_depth: u32,
}

struct Solver<'a> {
    game_data: &'a GameData,
    player_context: &'a PlayerContext,
    skills: &'a SkillUniverse,
    memo: HashMap<StateKey, Vec<Route>>,
    expanded_states: u32,
    skill_assignments: u32,
    route_combinations: u32,
}

impl Solver<'_> {
    fn solve_exact(
        &mut self,
        demon: DemonId,
        required_skills: SkillMask,
        exact_fusion_depth: u32,
    ) -> Result<Vec<Route>, SearchSafetyLimit> {
        let key = StateKey {
            demon,
            required_skills,
            exact_fusion_depth,
        };
        if let Some(routes) = self.memo.get(&key) {
            return Ok(routes.clone());
        }
        self.record_state_expansion()?;

        let Some(demon_meta) = self.game_data.demons().get(demon) else {
            self.memo.insert(key, Vec::new());
            return Ok(Vec::new());
        };
        if !is_available(demon_meta, self.player_context) {
            self.memo.insert(key, Vec::new());
            return Ok(Vec::new());
        }
        let base_level = demon_meta.base_level;
        let natural_skills = demon_meta.natural_skills.clone();
        let levels = self.meaningful_levels(base_level, &natural_skills, required_skills);
        let mut routes = Vec::new();

        if exact_fusion_depth == 0 {
            if let Some(route) =
                Self::make_upgrade_only_route(demon, required_skills, base_level, &levels)
            {
                routes.push(route);
            }
        } else {
            let direct_recipes = self
                .player_context
                .get_direct_recipes(demon)
                .unwrap_or_default()
                .to_vec();
            let inheritance_options = levels
                .into_iter()
                .filter_map(|(target_level, local_skills)| {
                    let skills_to_inherit = required_skills & !local_skills;
                    self.are_inheritable(skills_to_inherit)
                        .then_some((target_level, skills_to_inherit))
                })
                .collect::<Vec<_>>();
            let fusion_inputs =
                inheritance_options
                    .into_iter()
                    .flat_map(|(target_level, skills_to_inherit)| {
                        direct_recipes.iter().flat_map(move |recipe| {
                            enumerate_skill_assignments(skills_to_inherit, recipe.materials.len())
                                .into_iter()
                                .map(move |material_skills| (target_level, recipe, material_skills))
                        })
                    });
            for (target_level, recipe, material_skills) in fusion_inputs {
                self.record_skill_assignment()?;
                let mut material_route_options = Vec::with_capacity(recipe.materials.len());
                for (&material, &required) in recipe.materials.iter().zip(&material_skills) {
                    let mut options = Vec::new();
                    for child_depth in 0..exact_fusion_depth {
                        for route in self.solve_exact(material, required, child_depth)? {
                            options.push((child_depth, route));
                        }
                    }
                    if options.is_empty() {
                        material_route_options.clear();
                        break;
                    }
                    material_route_options.push(options);
                }
                if material_route_options.is_empty() {
                    continue;
                }

                let mut indices = vec![0; material_route_options.len()];
                loop {
                    self.record_route_combination()?;
                    if let Some(route) = fusion_route_for_combination(
                        self.skills,
                        recipe,
                        &material_skills,
                        &material_route_options,
                        &indices,
                        exact_fusion_depth,
                    ) {
                        routes.push(Self::add_upgrades(base_level, target_level, route));
                    }
                    if !advance_combination(&mut indices, &material_route_options) {
                        break;
                    }
                }
            }
        }

        self.memo.insert(key, routes.clone());
        Ok(routes)
    }

    fn make_upgrade_only_route(
        demon: DemonId,
        required_skills: SkillMask,
        base_level: u32,
        levels: &[(u32, SkillMask)],
    ) -> Option<Route> {
        let target_level = levels
            .iter()
            .find(|(_, local)| required_skills & !local == 0)?
            .0;
        Some(Self::add_upgrades(
            base_level,
            target_level,
            Route::Direct { demon },
        ))
    }

    fn meaningful_levels(
        &self,
        base_level: u32,
        natural_skills: &[NaturalSkill],
        required_skills: SkillMask,
    ) -> Vec<(u32, SkillMask)> {
        let mut levels = vec![base_level];
        for natural_skill in natural_skills {
            let SkillAcquisition::Level { level } = natural_skill.acquisition else {
                continue;
            };
            if required_skills & self.skills.bit(natural_skill.skill) != 0 {
                levels.push(level);
            }
        }
        levels.sort_unstable();
        levels.dedup();

        let mut result = Vec::new();
        for level in levels {
            let local_skills = self.local_skills(natural_skills, level);
            if result
                .last()
                .is_none_or(|(_, previous)| *previous != local_skills)
            {
                result.push((level, local_skills));
            }
        }
        result
    }

    fn local_skills(&self, natural_skills: &[NaturalSkill], target_level: u32) -> SkillMask {
        natural_skills.iter().fold(0, |mask, natural_skill| {
            let acquired = match natural_skill.acquisition {
                SkillAcquisition::Initial { .. } => true,
                SkillAcquisition::Level { level } => level <= target_level,
            };
            if acquired {
                mask | self.skills.bit(natural_skill.skill)
            } else {
                mask
            }
        })
    }

    fn are_inheritable(&self, skills: SkillMask) -> bool {
        self.skills.skill_ids(skills).iter().all(|skill| {
            self.game_data
                .skills()
                .get(*skill)
                .is_some_and(|skill| skill.inheritable)
        })
    }

    fn add_upgrades(base_level: u32, target_level: u32, mut previous: Route) -> Route {
        for level in base_level + 1..=target_level {
            previous = Route::Upgrade {
                level,
                previous: Box::new(previous),
            };
        }
        previous
    }

    fn record_state_expansion(&mut self) -> Result<(), SearchSafetyLimit> {
        if self.expanded_states >= MAX_EXPANDED_STATES {
            return Err(SearchSafetyLimit::ExpandedStates {
                maximum: MAX_EXPANDED_STATES,
            });
        }
        self.expanded_states += 1;
        Ok(())
    }

    fn record_skill_assignment(&mut self) -> Result<(), SearchSafetyLimit> {
        if self.skill_assignments >= MAX_SKILL_ASSIGNMENTS {
            return Err(SearchSafetyLimit::SkillAssignments {
                maximum: MAX_SKILL_ASSIGNMENTS,
            });
        }
        self.skill_assignments += 1;
        Ok(())
    }

    fn record_route_combination(&mut self) -> Result<(), SearchSafetyLimit> {
        if self.route_combinations >= MAX_ROUTE_COMBINATIONS {
            return Err(SearchSafetyLimit::RouteCombinations {
                maximum: MAX_ROUTE_COMBINATIONS,
            });
        }
        self.route_combinations += 1;
        Ok(())
    }
}
