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
type RoutesByDepth = Vec<Vec<Route>>;

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
    let routes_by_depth = solver
        .solve(request.target, required_skills, request.max_fusion_depth)
        .map_err(SearchError::SafetyLimitExceeded)?;
    let mut solutions = Vec::new();

    for (fusion_depth, routes) in (0..=request.max_fusion_depth).zip(routes_by_depth) {
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

const SKILL_MASK_COUNT: usize = 1 << SkillMask::BITS;
type FeasibleSkillMasks = [bool; SKILL_MASK_COUNT];

fn enumerate_skill_assignments(
    skills_to_inherit: SkillMask,
    feasible_masks_by_material: &[FeasibleSkillMasks],
) -> Vec<Vec<SkillMask>> {
    fn can_extend(
        assigned_skills: SkillMask,
        remaining_skills: SkillMask,
        feasible_masks: &FeasibleSkillMasks,
    ) -> bool {
        let mut additional_skills = remaining_skills;
        loop {
            if feasible_masks[usize::from(assigned_skills | additional_skills)] {
                return true;
            }
            if additional_skills == 0 {
                return false;
            }
            additional_skills = (additional_skills - 1) & remaining_skills;
        }
    }

    fn visit(
        skill_bits: &[SkillMask],
        skill_index: usize,
        remaining_skills: SkillMask,
        feasible_masks_by_material: &[FeasibleSkillMasks],
        material_skills: &mut [SkillMask],
        assignments: &mut Vec<Vec<SkillMask>>,
    ) {
        if skill_index == skill_bits.len() {
            assignments.push(material_skills.to_vec());
            return;
        }

        let skill = skill_bits[skill_index];
        let remaining_skills = remaining_skills & !skill;
        for material_index in 0..material_skills.len() {
            material_skills[material_index] |= skill;
            let can_complete = material_skills.iter().zip(feasible_masks_by_material).all(
                |(&assigned_skills, feasible_masks)| {
                    can_extend(assigned_skills, remaining_skills, feasible_masks)
                },
            );
            if can_complete {
                visit(
                    skill_bits,
                    skill_index + 1,
                    remaining_skills,
                    feasible_masks_by_material,
                    material_skills,
                    assignments,
                );
            }
            material_skills[material_index] &= !skill;
        }
    }

    if feasible_masks_by_material.is_empty() {
        return Vec::new();
    }
    let skill_bits = (0..SkillMask::BITS)
        .map(|index| 1_u8 << index)
        .filter(|bit| skills_to_inherit & bit != 0)
        .collect::<Vec<_>>();
    let mut assignments = Vec::new();
    visit(
        &skill_bits,
        0,
        skills_to_inherit,
        feasible_masks_by_material,
        &mut vec![0; feasible_masks_by_material.len()],
        &mut assignments,
    );
    assignments
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct StateKey {
    demon: DemonId,
    required_skills: SkillMask,
}

#[derive(Debug, Clone, Copy)]
struct CachedRouteHandle {
    state: StateKey,
    fusion_depth: u32,
    route_index: usize,
}

#[derive(Default)]
struct MemoEntry {
    routes_by_depth: RoutesByDepth,
}

fn visit_route_combinations<E>(
    material_route_options: &[Vec<CachedRouteHandle>],
    required_child_depth: u32,
    indices: &mut [usize],
    visit: &mut impl FnMut(&[usize]) -> Result<(), E>,
) -> Result<(), E> {
    let mut suffix_can_reach_required_depth = vec![false; material_route_options.len() + 1];
    for position in (0..material_route_options.len()).rev() {
        suffix_can_reach_required_depth[position] = suffix_can_reach_required_depth[position + 1]
            || material_route_options[position]
                .iter()
                .any(|option| option.fusion_depth == required_child_depth);
    }

    visit_route_combinations_from(
        material_route_options,
        required_child_depth,
        &suffix_can_reach_required_depth,
        0,
        false,
        indices,
        visit,
    )
}

fn visit_route_combinations_from<E>(
    material_route_options: &[Vec<CachedRouteHandle>],
    required_child_depth: u32,
    suffix_can_reach_required_depth: &[bool],
    position: usize,
    has_required_depth: bool,
    indices: &mut [usize],
    visit: &mut impl FnMut(&[usize]) -> Result<(), E>,
) -> Result<(), E> {
    if position == material_route_options.len() {
        return if has_required_depth {
            visit(indices)
        } else {
            Ok(())
        };
    }
    if !has_required_depth && !suffix_can_reach_required_depth[position] {
        return Ok(());
    }

    for (option_index, option) in material_route_options[position].iter().enumerate() {
        indices[position] = option_index;
        visit_route_combinations_from(
            material_route_options,
            required_child_depth,
            suffix_can_reach_required_depth,
            position + 1,
            has_required_depth || option.fusion_depth == required_child_depth,
            indices,
            visit,
        )?;
    }
    Ok(())
}

fn fusion_route_for_combination(
    skills: &SkillUniverse,
    memo: &HashMap<StateKey, MemoEntry>,
    recipe: &RecipeMeta,
    material_skills: &[SkillMask],
    material_route_options: &[Vec<CachedRouteHandle>],
    indices: &[usize],
) -> Route {
    let materials = material_skills
        .iter()
        .copied()
        .zip(material_route_options.iter().zip(indices))
        .map(|(required, (options, index))| {
            let selected = options[*index];
            let route = memo
                .get(&selected.state)
                .and_then(|entry| entry.routes_by_depth.get(selected.fusion_depth as usize))
                .and_then(|routes| routes.get(selected.route_index))
                .expect("cached route handle must resolve")
                .clone();
            FusionSubroute {
                required_skills: skills.skill_ids(required),
                route,
            }
        })
        .collect();
    Route::Fusion {
        recipe: recipe.clone(),
        materials,
    }
}

struct Solver<'a> {
    game_data: &'a GameData,
    player_context: &'a PlayerContext,
    skills: &'a SkillUniverse,
    memo: HashMap<StateKey, MemoEntry>,
    expanded_states: u32,
    skill_assignments: u32,
    route_combinations: u32,
}

impl Solver<'_> {
    fn solve(
        &mut self,
        demon: DemonId,
        required_skills: SkillMask,
        maximum_fusion_depth: u32,
    ) -> Result<RoutesByDepth, SearchSafetyLimit> {
        let key = StateKey {
            demon,
            required_skills,
        };
        self.ensure_through(key, maximum_fusion_depth)?;
        Ok(self
            .memo
            .remove(&key)
            .expect("root state must be cached")
            .routes_by_depth)
    }

    fn ensure_through(
        &mut self,
        key: StateKey,
        maximum_fusion_depth: u32,
    ) -> Result<(), SearchSafetyLimit> {
        self.memo.entry(key).or_default();
        loop {
            let next_depth = u32::try_from(
                self.memo
                    .get(&key)
                    .expect("state must be cached")
                    .routes_by_depth
                    .len(),
            )
            .expect("cached depth count must fit in u32");
            if next_depth > maximum_fusion_depth {
                return Ok(());
            }

            self.record_state_expansion()?;
            let routes = self.solve_layer(key, next_depth)?;
            self.memo
                .get_mut(&key)
                .expect("state must be cached")
                .routes_by_depth
                .push(routes);
        }
    }

    fn solve_layer(
        &mut self,
        key: StateKey,
        exact_fusion_depth: u32,
    ) -> Result<Vec<Route>, SearchSafetyLimit> {
        let Some(demon_meta) = self.game_data.demons().get(key.demon) else {
            return Ok(Vec::new());
        };
        if !is_available(demon_meta, self.player_context) {
            return Ok(Vec::new());
        }
        let base_level = demon_meta.base_level;
        let natural_skills = demon_meta.natural_skills.clone();
        let levels = self.meaningful_levels(base_level, &natural_skills, key.required_skills);

        if exact_fusion_depth == 0 {
            return Ok(Self::make_upgrade_only_route(
                key.demon,
                key.required_skills,
                base_level,
                &levels,
            )
            .into_iter()
            .collect());
        }

        let direct_recipes = self
            .player_context
            .get_direct_recipes(key.demon)
            .unwrap_or_default()
            .to_vec();
        let inheritance_options = levels
            .into_iter()
            .filter_map(|(target_level, local_skills)| {
                let skills_to_inherit = key.required_skills & !local_skills;
                self.are_inheritable(skills_to_inherit)
                    .then_some((target_level, skills_to_inherit))
            })
            .collect::<Vec<_>>();
        let mut routes = Vec::new();

        for (target_level, skills_to_inherit) in inheritance_options {
            for recipe in &direct_recipes {
                let Some(feasible_masks_by_material) = self.feasible_material_skill_masks(
                    &recipe.materials,
                    skills_to_inherit,
                    exact_fusion_depth - 1,
                )?
                else {
                    continue;
                };
                for material_skills in
                    enumerate_skill_assignments(skills_to_inherit, &feasible_masks_by_material)
                {
                    self.record_skill_assignment()?;
                    let mut material_route_options = Vec::with_capacity(recipe.materials.len());
                    for (&material, &required) in recipe.materials.iter().zip(&material_skills) {
                        let child_state = StateKey {
                            demon: material,
                            required_skills: required,
                        };
                        self.ensure_through(child_state, exact_fusion_depth - 1)?;
                        let child_routes_by_depth = &self
                            .memo
                            .get(&child_state)
                            .expect("child state must be cached")
                            .routes_by_depth;
                        let options = (0..exact_fusion_depth)
                            .zip(child_routes_by_depth)
                            .flat_map(|(fusion_depth, routes)| {
                                (0..routes.len()).map(move |route_index| CachedRouteHandle {
                                    state: child_state,
                                    fusion_depth,
                                    route_index,
                                })
                            })
                            .collect::<Vec<_>>();
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
                    visit_route_combinations(
                        &material_route_options,
                        exact_fusion_depth - 1,
                        &mut indices,
                        &mut |indices| {
                            self.record_route_combination()?;
                            let route = fusion_route_for_combination(
                                self.skills,
                                &self.memo,
                                recipe,
                                &material_skills,
                                &material_route_options,
                                indices,
                            );
                            routes.push(Self::add_upgrades(base_level, target_level, route));
                            Ok(())
                        },
                    )?;
                }
            }
        }

        Ok(routes)
    }

    fn feasible_material_skill_masks(
        &mut self,
        materials: &[DemonId],
        skills_to_inherit: SkillMask,
        maximum_child_depth: u32,
    ) -> Result<Option<Vec<FeasibleSkillMasks>>, SearchSafetyLimit> {
        let layer_count =
            usize::try_from(maximum_child_depth).expect("fusion depth must fit in usize") + 1;
        let mut feasible_masks_by_material = Vec::with_capacity(materials.len());
        for &material in materials {
            let mut feasible_masks = [false; SKILL_MASK_COUNT];
            feasible_masks[0] = true;
            let mut skills = skills_to_inherit;
            while skills != 0 {
                let state = StateKey {
                    demon: material,
                    required_skills: skills,
                };
                self.ensure_through(state, maximum_child_depth)?;
                feasible_masks[usize::from(skills)] = self
                    .memo
                    .get(&state)
                    .expect("material state must be cached")
                    .routes_by_depth
                    .iter()
                    .take(layer_count)
                    .any(|routes| !routes.is_empty());
                skills = (skills - 1) & skills_to_inherit;
            }
            feasible_masks_by_material.push(feasible_masks);
        }

        for skill_index in 0..SkillMask::BITS {
            let skill = 1_u8 << skill_index;
            if skills_to_inherit & skill != 0
                && !feasible_masks_by_material
                    .iter()
                    .any(|feasible_masks| feasible_masks[usize::from(skill)])
            {
                return Ok(None);
            }
        }
        Ok(Some(feasible_masks_by_material))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skill_assignments_keep_only_extendable_material_choices() {
        let mut first_material = [false; SKILL_MASK_COUNT];
        first_material[0] = true;
        first_material[1] = true;
        first_material[3] = true;
        let mut second_material = [false; SKILL_MASK_COUNT];
        second_material[0] = true;
        second_material[2] = true;

        assert_eq!(
            enumerate_skill_assignments(3, &[first_material, second_material]),
            vec![vec![3, 0], vec![1, 2]]
        );
    }
}
