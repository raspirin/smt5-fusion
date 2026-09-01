use crate::{
    data::game_data::GameData,
    forward_fuse::{fuse, is_available},
    model::{
        demon::{Demon, DemonId, DemonMeta, SkillAcquisition},
        player_context::PlayerContext,
        recipe::RecipeMeta,
        route::{FusionSubroute, Route},
        skill::{SkillCategory, SkillId},
    },
    reverse_search::SearchRequest,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplayError {
    UnknownDemon(DemonId),
    UnknownSkill(SkillId),
    UnsupportedSkill(SkillId),
    UnavailableDemon(DemonId),
    UnexpectedDemon { expected: DemonId, actual: DemonId },
    TooManySkills { selected: usize, maximum: usize },
    FusionDepthExceeded { actual: u32, maximum: u32 },
    InvalidDirect(DemonId),
    InvalidUpgrade { demon: DemonId, level: u32 },
    InvalidFusion(DemonId),
}

pub fn replay(
    game_data: &GameData,
    player_context: &PlayerContext,
    request: &SearchRequest,
    route: &Route,
) -> Result<Demon, ReplayError> {
    let target = game_data
        .demons()
        .get(request.target)
        .ok_or(ReplayError::UnknownDemon(request.target))?;
    if !is_available(target, player_context) {
        return Err(ReplayError::UnavailableDemon(request.target));
    }
    let required_skills = normalize_required_skills(game_data, &request.required_skills)?;
    let (demon, fusion_depth) = Replayer {
        game_data,
        player_context,
    }
    .replay(request.target, &required_skills, route)?;
    if fusion_depth > request.max_fusion_depth {
        return Err(ReplayError::FusionDepthExceeded {
            actual: fusion_depth,
            maximum: request.max_fusion_depth,
        });
    }
    Ok(demon)
}

struct Replayer<'a> {
    game_data: &'a GameData,
    player_context: &'a PlayerContext,
}

impl Replayer<'_> {
    fn replay(
        &self,
        expected_demon: DemonId,
        required_skills: &[SkillId],
        route: &Route,
    ) -> Result<(Demon, u32), ReplayError> {
        let actual_demon = route_demon(route);
        if actual_demon != expected_demon {
            return Err(ReplayError::UnexpectedDemon {
                expected: expected_demon,
                actual: actual_demon,
            });
        }
        let demon_meta = self
            .game_data
            .demons()
            .get(expected_demon)
            .ok_or(ReplayError::UnknownDemon(expected_demon))?;
        if !is_available(demon_meta, self.player_context) {
            return Err(ReplayError::UnavailableDemon(expected_demon));
        }

        match route {
            Route::Direct { .. } => self.replay_direct(demon_meta, required_skills),
            Route::Upgrade { level, previous } => {
                self.replay_upgrade(demon_meta, required_skills, *level, previous)
            }
            Route::Fusion { recipe, materials } => {
                self.replay_fusion(demon_meta, required_skills, recipe, materials)
            }
        }
    }

    fn replay_direct(
        &self,
        demon_meta: &DemonMeta,
        required_skills: &[SkillId],
    ) -> Result<(Demon, u32), ReplayError> {
        let skills = initial_skills(demon_meta);
        if !contains_all(&skills, required_skills) || skills.len() > Demon::MAX_SKILLS {
            return Err(ReplayError::InvalidDirect(demon_meta.id));
        }
        Ok((
            Demon {
                meta: demon_meta.id,
                level: demon_meta.base_level,
                skills,
            },
            0,
        ))
    }

    fn replay_upgrade(
        &self,
        demon_meta: &DemonMeta,
        required_skills: &[SkillId],
        level: u32,
        previous_route: &Route,
    ) -> Result<(Demon, u32), ReplayError> {
        let learned_skills = learned_skills(demon_meta, level);
        let previous_required = required_skills
            .iter()
            .copied()
            .filter(|skill| !learned_skills.contains(skill))
            .collect::<Vec<_>>();
        let (previous, fusion_depth) =
            self.replay(demon_meta.id, &previous_required, previous_route)?;
        if previous.level.checked_add(1) != Some(level) || level > 99 {
            return Err(ReplayError::InvalidUpgrade {
                demon: demon_meta.id,
                level,
            });
        }

        let mut skills = previous.skills;
        for skill in learned_skills {
            if !skills.contains(&skill) {
                skills.push(skill);
            }
        }
        trim_to_capacity(&mut skills, required_skills)?;
        if !contains_all(&skills, required_skills) {
            return Err(ReplayError::InvalidUpgrade {
                demon: demon_meta.id,
                level,
            });
        }

        Ok((
            Demon {
                meta: demon_meta.id,
                level,
                skills,
            },
            fusion_depth,
        ))
    }

    fn replay_fusion(
        &self,
        demon_meta: &DemonMeta,
        required_skills: &[SkillId],
        recipe_meta: &RecipeMeta,
        subroutes: &[FusionSubroute],
    ) -> Result<(Demon, u32), ReplayError> {
        if recipe_meta.result != demon_meta.id
            || recipe_meta.materials.len() != subroutes.len()
            || !self
                .player_context
                .get_direct_recipes(demon_meta.id)
                .is_some_and(|recipes| recipes.contains(recipe_meta))
        {
            return Err(ReplayError::InvalidFusion(demon_meta.id));
        }
        if !recipe_meta.is_special {
            let [left, right] = recipe_meta.materials.as_slice() else {
                return Err(ReplayError::InvalidFusion(demon_meta.id));
            };
            if fuse(self.game_data, self.player_context, *left, *right) != Some(demon_meta.id) {
                return Err(ReplayError::InvalidFusion(demon_meta.id));
            }
        }

        let initial_skills = initial_skills(demon_meta);
        let expected_inherited = required_skills
            .iter()
            .copied()
            .filter(|skill| !initial_skills.contains(skill))
            .collect::<Vec<_>>();
        let mut assigned_skills = Vec::new();
        for subroute in subroutes {
            for skill in
                self.normalize_inherited_skills(demon_meta.id, &subroute.required_skills)?
            {
                if assigned_skills.contains(&skill) {
                    return Err(ReplayError::InvalidFusion(demon_meta.id));
                }
                assigned_skills.push(skill);
            }
        }
        assigned_skills.sort_unstable();
        if assigned_skills != expected_inherited {
            return Err(ReplayError::InvalidFusion(demon_meta.id));
        }

        let mut maximum_material_depth = 0;
        for (material, subroute) in recipe_meta.materials.iter().zip(subroutes) {
            let (_, depth) = self.replay(*material, &subroute.required_skills, &subroute.route)?;
            maximum_material_depth = maximum_material_depth.max(depth);
        }

        let mut skills = initial_skills;
        for subroute in subroutes {
            for &skill in &subroute.required_skills {
                if !skills.contains(&skill) {
                    skills.push(skill);
                }
            }
        }
        trim_to_capacity(&mut skills, required_skills)?;
        if !contains_all(&skills, required_skills) {
            return Err(ReplayError::InvalidFusion(demon_meta.id));
        }

        Ok((
            Demon {
                meta: demon_meta.id,
                level: demon_meta.base_level,
                skills,
            },
            maximum_material_depth + 1,
        ))
    }

    fn normalize_inherited_skills(
        &self,
        result: DemonId,
        skills: &[SkillId],
    ) -> Result<Vec<SkillId>, ReplayError> {
        let mut normalized = skills.to_vec();
        normalized.sort_unstable();
        if normalized.as_slice() != skills || normalized.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(ReplayError::InvalidFusion(result));
        }
        for &skill_id in &normalized {
            let skill = self
                .game_data
                .skills()
                .get(skill_id)
                .ok_or(ReplayError::UnknownSkill(skill_id))?;
            if !skill.inheritable {
                return Err(ReplayError::InvalidFusion(result));
            }
        }
        Ok(normalized)
    }
}

fn normalize_required_skills(
    game_data: &GameData,
    required_skills: &[SkillId],
) -> Result<Vec<SkillId>, ReplayError> {
    let mut skills = required_skills.to_vec();
    skills.sort_unstable();
    skills.dedup();
    if skills.len() > Demon::MAX_SKILLS {
        return Err(ReplayError::TooManySkills {
            selected: skills.len(),
            maximum: Demon::MAX_SKILLS,
        });
    }
    for &skill_id in &skills {
        let skill = game_data
            .skills()
            .get(skill_id)
            .ok_or(ReplayError::UnknownSkill(skill_id))?;
        if skill.category == SkillCategory::Innate || skill.flags.magatsuhi || skill.flags.item_only
        {
            return Err(ReplayError::UnsupportedSkill(skill_id));
        }
    }
    Ok(skills)
}

fn route_demon(route: &Route) -> DemonId {
    match route {
        Route::Direct { demon } => *demon,
        Route::Upgrade { previous, .. } => route_demon(previous),
        Route::Fusion { recipe, .. } => recipe.result,
    }
}

fn initial_skills(demon: &DemonMeta) -> Vec<SkillId> {
    let mut skills = demon
        .natural_skills
        .iter()
        .filter_map(|natural_skill| match natural_skill.acquisition {
            SkillAcquisition::Initial { order } => Some((order, natural_skill.skill)),
            SkillAcquisition::Level { .. } => None,
        })
        .collect::<Vec<_>>();
    skills.sort_unstable();
    skills.into_iter().map(|(_, skill)| skill).collect()
}

fn learned_skills(demon: &DemonMeta, level: u32) -> Vec<SkillId> {
    demon
        .natural_skills
        .iter()
        .filter_map(|natural_skill| match natural_skill.acquisition {
            SkillAcquisition::Level {
                level: learned_level,
            } if learned_level == level => Some(natural_skill.skill),
            _ => None,
        })
        .collect()
}

fn contains_all(skills: &[SkillId], required_skills: &[SkillId]) -> bool {
    required_skills.iter().all(|skill| skills.contains(skill))
}

fn trim_to_capacity(
    skills: &mut Vec<SkillId>,
    required_skills: &[SkillId],
) -> Result<(), ReplayError> {
    while skills.len() > Demon::MAX_SKILLS {
        let Some(index) = skills
            .iter()
            .rposition(|skill| !required_skills.contains(skill))
        else {
            return Err(ReplayError::TooManySkills {
                selected: skills.len(),
                maximum: Demon::MAX_SKILLS,
            });
        };
        skills.remove(index);
    }
    Ok(())
}
