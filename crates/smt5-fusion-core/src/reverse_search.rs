use std::{collections::HashMap, rc::Rc};

use crate::{
    data::game_data::GameData,
    forward_fuse::is_available,
    model::{
        demon::{Demon, DemonId, NaturalSkill, SkillAcquisition},
        player_context::PlayerContext,
        recipe::RecipeMeta,
        route::Route,
        route_space::{
            DirectChoice, FusionChoice, FusionMaterialOptions, RouteChoice, RouteSelector,
            RouteSpace,
        },
        skill::{SkillCategory, SkillId},
    },
};

const MAX_EXPANDED_STATES: u32 = 131_072;
const MAX_SKILL_ASSIGNMENTS: u32 = 10_000_000;

type SkillMask = u8;
type SpacesByDepth = Vec<Rc<RouteSpace>>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchRequest {
    pub target: DemonId,
    pub required_skills: Vec<SkillId>,
    pub max_fusion_depth: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchSolution {
    pub route: Rc<Route>,
    pub demon: Demon,
    pub fusion_depth: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchSafetyLimit {
    ExpandedStates { maximum: u32 },
    SkillAssignments { maximum: u32 },
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
) -> Result<RouteSelector, SearchError> {
    let target = game_data
        .demons()
        .get(request.target)
        .ok_or(SearchError::UnknownDemon(request.target))?;
    if !is_available(target, player_context) {
        return Err(SearchError::UnavailableTarget(request.target));
    }
    let skills = SkillUniverse::new(game_data, &request.required_skills)?;
    let required_skills = skills.full_mask();
    let routes = Solver {
        game_data,
        player_context,
        skills: &skills,
        memo: HashMap::new(),
        route_spaces_by_state: HashMap::new(),
        recipes_by_demon: HashMap::new(),
        expanded_states: 0,
        skill_assignments: 0,
    }
    .solve(request.target, required_skills, request.max_fusion_depth)
    .map_err(SearchError::SafetyLimitExceeded)?;

    Ok(RouteSelector::new(game_data, player_context, routes))
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

const SKILL_MASK_COUNT: usize = 1_usize << SkillMask::BITS;
const FEASIBLE_SKILL_MASK_WORDS: usize = SKILL_MASK_COUNT / u64::BITS as usize;

#[derive(Clone, Copy, Default)]
struct FeasibleSkillMasks([u64; FEASIBLE_SKILL_MASK_WORDS]);

impl FeasibleSkillMasks {
    fn insert(&mut self, mask: SkillMask) {
        let mask = usize::from(mask);
        self.0[mask / u64::BITS as usize] |= 1_u64 << (mask % u64::BITS as usize);
    }

    fn contains(&self, mask: SkillMask) -> bool {
        let mask = usize::from(mask);
        self.0[mask / u64::BITS as usize] & (1_u64 << (mask % u64::BITS as usize)) != 0
    }
}

fn for_each_skill_assignment<E>(
    skills_to_inherit: SkillMask,
    feasible_masks_by_material: &[FeasibleSkillMasks],
    material_skills: &mut [SkillMask],
    mut visitor: impl FnMut(&[SkillMask]) -> Result<(), E>,
) -> Result<(), E> {
    fn can_extend(
        assigned_skills: SkillMask,
        remaining_skills: SkillMask,
        feasible_masks: &FeasibleSkillMasks,
    ) -> bool {
        let mut additional_skills = remaining_skills;
        loop {
            if feasible_masks.contains(assigned_skills | additional_skills) {
                return true;
            }
            if additional_skills == 0 {
                return false;
            }
            additional_skills = (additional_skills - 1) & remaining_skills;
        }
    }

    fn visit<E>(
        remaining_skills: SkillMask,
        feasible_masks_by_material: &[FeasibleSkillMasks],
        material_skills: &mut [SkillMask],
        visitor: &mut impl FnMut(&[SkillMask]) -> Result<(), E>,
    ) -> Result<(), E> {
        if remaining_skills == 0 {
            return visitor(material_skills);
        }

        let skill = 1_u8 << remaining_skills.trailing_zeros();
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
                    remaining_skills,
                    feasible_masks_by_material,
                    material_skills,
                    visitor,
                )?;
            }
            material_skills[material_index] &= !skill;
        }
        Ok(())
    }

    material_skills.fill(0);
    if skills_to_inherit == 0 {
        return visitor(material_skills);
    }
    debug_assert_eq!(feasible_masks_by_material.len(), material_skills.len());
    visit(
        skills_to_inherit,
        feasible_masks_by_material,
        material_skills,
        &mut visitor,
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct StateKey {
    demon: DemonId,
    required_skills: SkillMask,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct BoundedStateKey {
    state: StateKey,
    maximum_fusion_depth: u32,
}

#[derive(Default)]
struct MemoEntry {
    spaces_by_depth: SpacesByDepth,
}

struct Solver<'a> {
    game_data: &'a GameData,
    player_context: &'a PlayerContext,
    skills: &'a SkillUniverse,
    memo: HashMap<StateKey, MemoEntry>,
    route_spaces_by_state: HashMap<BoundedStateKey, Rc<[Rc<RouteSpace>]>>,
    recipes_by_demon: HashMap<DemonId, Rc<[Rc<RecipeMeta>]>>,
    expanded_states: u32,
    skill_assignments: u32,
}

impl Solver<'_> {
    fn solve(
        mut self,
        demon: DemonId,
        required_skills: SkillMask,
        maximum_fusion_depth: u32,
    ) -> Result<SpacesByDepth, SearchSafetyLimit> {
        let key = StateKey {
            demon,
            required_skills,
        };
        self.ensure_through(key, maximum_fusion_depth)?;
        Ok(self
            .memo
            .remove(&key)
            .expect("root state must be cached")
            .spaces_by_depth)
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
                    .spaces_by_depth
                    .len(),
            )
            .expect("cached depth count must fit in u32");
            if next_depth > maximum_fusion_depth {
                return Ok(());
            }

            self.record_state_expansion()?;
            let space = self.solve_layer(key, next_depth)?;
            self.memo
                .get_mut(&key)
                .expect("state must be cached")
                .spaces_by_depth
                .push(space);
        }
    }

    fn route_spaces_through(
        &mut self,
        state: StateKey,
        maximum_fusion_depth: u32,
    ) -> Result<Rc<[Rc<RouteSpace>]>, SearchSafetyLimit> {
        let key = BoundedStateKey {
            state,
            maximum_fusion_depth,
        };
        if let Some(routes) = self.route_spaces_by_state.get(&key) {
            return Ok(Rc::clone(routes));
        }

        self.ensure_through(state, maximum_fusion_depth)?;
        let layer_count =
            usize::try_from(maximum_fusion_depth).expect("fusion depth must fit in usize") + 1;
        let routes = self
            .memo
            .get(&state)
            .expect("state must be cached")
            .spaces_by_depth
            .iter()
            .take(layer_count)
            .cloned()
            .collect::<Vec<_>>()
            .into();
        self.route_spaces_by_state.insert(key, Rc::clone(&routes));
        Ok(routes)
    }

    fn solve_layer(
        &mut self,
        key: StateKey,
        exact_fusion_depth: u32,
    ) -> Result<Rc<RouteSpace>, SearchSafetyLimit> {
        let mut choices = Vec::new();
        let Some(demon_meta) = self.game_data.demons().get(key.demon) else {
            return Ok(self.route_space(key, exact_fusion_depth, choices));
        };
        if !is_available(demon_meta, self.player_context) {
            return Ok(self.route_space(key, exact_fusion_depth, choices));
        }
        let base_level = demon_meta.base_level;
        let levels =
            self.meaningful_levels(base_level, &demon_meta.natural_skills, key.required_skills);

        if exact_fusion_depth == 0 {
            if let Some(target_level) = Self::direct_target_level(key.required_skills, &levels) {
                choices.push(RouteChoice::Direct(DirectChoice { target_level }));
            }
            return Ok(self.route_space(key, exact_fusion_depth, choices));
        }

        let direct_recipes = self.shared_direct_recipes(key.demon);
        let mut feasible_masks_by_material = Vec::new();
        let mut material_skills = Vec::new();
        for (target_level, local_skills) in levels {
            let skills_to_inherit = key.required_skills & !local_skills;
            if !self.are_inheritable(skills_to_inherit) {
                continue;
            }
            for recipe in direct_recipes.iter() {
                if !self.feasible_material_skill_masks(
                    &recipe.materials,
                    skills_to_inherit,
                    exact_fusion_depth - 1,
                    &mut feasible_masks_by_material,
                )? {
                    continue;
                }
                material_skills.resize(recipe.materials.len(), 0);
                for_each_skill_assignment(
                    skills_to_inherit,
                    &feasible_masks_by_material,
                    &mut material_skills,
                    |material_skills| {
                        let mut materials = Vec::with_capacity(recipe.materials.len());
                        let mut reaches_required_depth = false;
                        for (&material, &required) in recipe.materials.iter().zip(material_skills) {
                            let child_state = StateKey {
                                demon: material,
                                required_skills: required,
                            };
                            let routes =
                                self.route_spaces_through(child_state, exact_fusion_depth - 1)?;
                            debug_assert!(routes.iter().any(|space| !space.choices.is_empty()));
                            reaches_required_depth |= routes
                                .get((exact_fusion_depth - 1) as usize)
                                .is_some_and(|space| !space.choices.is_empty());
                            materials.push(FusionMaterialOptions {
                                required_skills: self.skills.skill_ids(required),
                                routes,
                            });
                        }
                        if materials.is_empty() || !reaches_required_depth {
                            return Ok(());
                        }
                        self.record_skill_assignment()?;
                        choices.push(RouteChoice::Fusion(FusionChoice {
                            target_level,
                            recipe: Rc::clone(recipe),
                            materials,
                        }));
                        Ok(())
                    },
                )?;
            }
        }

        Ok(self.route_space(key, exact_fusion_depth, choices))
    }

    fn shared_direct_recipes(&mut self, demon: DemonId) -> Rc<[Rc<RecipeMeta>]> {
        if let Some(recipes) = self.recipes_by_demon.get(&demon) {
            return Rc::clone(recipes);
        }
        let recipes = self
            .player_context
            .get_direct_recipes(demon)
            .unwrap_or_default()
            .iter()
            .cloned()
            .map(Rc::new)
            .collect::<Vec<_>>()
            .into();
        self.recipes_by_demon.insert(demon, Rc::clone(&recipes));
        recipes
    }

    fn route_space(
        &self,
        key: StateKey,
        fusion_depth: u32,
        choices: Vec<RouteChoice>,
    ) -> Rc<RouteSpace> {
        RouteSpace::new(
            self.game_data,
            key.demon,
            self.skills.skill_ids(key.required_skills),
            fusion_depth,
            choices,
        )
    }

    fn feasible_material_skill_masks(
        &mut self,
        materials: &[DemonId],
        skills_to_inherit: SkillMask,
        maximum_child_depth: u32,
        feasible_masks_by_material: &mut Vec<FeasibleSkillMasks>,
    ) -> Result<bool, SearchSafetyLimit> {
        feasible_masks_by_material.clear();
        if skills_to_inherit == 0 {
            return Ok(true);
        }

        let layer_count =
            usize::try_from(maximum_child_depth).expect("fusion depth must fit in usize") + 1;
        for &material in materials {
            let mut feasible_masks = FeasibleSkillMasks::default();
            feasible_masks.insert(0);
            let mut skills = skills_to_inherit;
            while skills != 0 {
                let state = StateKey {
                    demon: material,
                    required_skills: skills,
                };
                self.ensure_through(state, maximum_child_depth)?;
                if self
                    .memo
                    .get(&state)
                    .expect("material state must be cached")
                    .spaces_by_depth
                    .iter()
                    .take(layer_count)
                    .any(|space| !space.choices.is_empty())
                {
                    feasible_masks.insert(skills);
                }
                skills = (skills - 1) & skills_to_inherit;
            }
            feasible_masks_by_material.push(feasible_masks);
        }

        for skill_index in 0..SkillMask::BITS {
            let skill = 1_u8 << skill_index;
            if skills_to_inherit & skill != 0
                && !feasible_masks_by_material
                    .iter()
                    .any(|feasible_masks| feasible_masks.contains(skill))
            {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn direct_target_level(required_skills: SkillMask, levels: &[(u32, SkillMask)]) -> Option<u32> {
        levels
            .iter()
            .find(|(_, local)| required_skills & !local == 0)
            .map(|(level, _)| *level)
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
        self.skills.skills.iter().enumerate().all(|(index, skill)| {
            skills & (1_u8 << index) == 0
                || self
                    .game_data
                    .skills()
                    .get(*skill)
                    .is_some_and(|skill| skill.inheritable)
        })
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dataset::{self, demon_ids, skill_ids};

    #[test]
    fn skill_assignments_keep_only_extendable_material_choices() {
        let mut first_material = FeasibleSkillMasks::default();
        first_material.insert(0);
        first_material.insert(1);
        first_material.insert(3);
        let mut second_material = FeasibleSkillMasks::default();
        second_material.insert(0);
        second_material.insert(2);
        let mut material_skills = [0; 2];
        let mut assignments = Vec::new();

        for_each_skill_assignment(
            3,
            &[first_material, second_material],
            &mut material_skills,
            |assignment| {
                assignments.push(assignment.to_vec());
                Ok::<_, ()>(())
            },
        )
        .unwrap();

        assert_eq!(assignments, vec![vec![3, 0], vec![1, 2]]);
    }

    #[test]
    fn empty_skill_assignment_is_visited_once() {
        let mut material_skills = [SkillMask::MAX; 3];
        let mut assignments = Vec::new();

        for_each_skill_assignment(0, &[], &mut material_skills, |assignment| {
            assignments.push(assignment.to_vec());
            Ok::<_, ()>(())
        })
        .unwrap();

        assert_eq!(assignments, vec![vec![0, 0, 0]]);
    }

    #[test]
    fn repeated_choices_share_recipes_and_material_route_lists() {
        let data = dataset::game_data();
        let mut context = PlayerContext::default();
        context.set_konohana_sakuya_dlc(true);
        context.set_dagda_dlc(true);
        context.prepare_direct_recipes(&data);
        let selector = search(
            &data,
            &context,
            &SearchRequest {
                target: demon_ids::PIXIE,
                required_skills: vec![skill_ids::AGI],
                max_fusion_depth: 2,
            },
        )
        .unwrap();
        let choices = selector.routes[2]
            .choices
            .iter()
            .filter_map(|choice| match choice {
                RouteChoice::Direct(_) => None,
                RouteChoice::Fusion(fusion) => Some(fusion),
            })
            .collect::<Vec<_>>();

        assert!(choices.iter().enumerate().any(|(index, left)| {
            choices[index + 1..]
                .iter()
                .any(|right| Rc::ptr_eq(&left.recipe, &right.recipe))
        }));
        assert!(selector.routes[1].choices.iter().any(|left| {
            let RouteChoice::Fusion(left) = left else {
                return false;
            };
            choices
                .iter()
                .any(|right| left.recipe == right.recipe && Rc::ptr_eq(&left.recipe, &right.recipe))
        }));
        assert!(choices.iter().enumerate().any(|(index, left)| {
            choices[index + 1..].iter().any(|right| {
                left.materials.iter().any(|left_material| {
                    right.materials.iter().any(|right_material| {
                        Rc::ptr_eq(&left_material.routes, &right_material.routes)
                    })
                })
            })
        }));
    }
}
