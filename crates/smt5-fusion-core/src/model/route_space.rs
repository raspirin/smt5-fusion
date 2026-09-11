use std::rc::Rc;

use num_bigint::BigUint;

use crate::{
    data::game_data::GameData,
    reverse_search::SearchRequest,
    route_ranking::{self, BestChoice},
    route_replay,
};

pub use crate::route_ranking::RouteScore;

use super::{
    demon::{Demon, DemonId},
    player_context::PlayerContext,
    recipe::RecipeMeta,
    route::{FusionSubroute, Route},
    skill::SkillId,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteSelector {
    pub routes: Vec<Rc<RouteSpace>>,
    pub default_selection: Option<RouteSelection>,
    pub route_count: BigUint,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteSpace {
    pub demon: DemonId,
    pub required_skills: Vec<SkillId>,
    pub fusion_depth: u32,
    pub choices: Vec<RouteChoice>,
    pub route_count: BigUint,
    best_choice: Option<BestChoice>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouteChoice {
    Direct(DirectChoice),
    Fusion(FusionChoice),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectChoice {
    pub target_level: u32,
    pub estimated_macca: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FusionChoice {
    pub target_level: u32,
    pub recipe: Rc<RecipeMeta>,
    pub materials: Vec<FusionMaterialOptions>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FusionMaterialOptions {
    pub required_skills: Vec<SkillId>,
    pub routes: Rc<[Rc<RouteSpace>]>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RoutePath(pub Vec<usize>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteSelection {
    pub space: Rc<RouteSpace>,
    pub choice_index: usize,
    pub demon: Demon,
    pub materials: Vec<RouteSelection>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteUpdate {
    pub replace_from: RoutePath,
    pub new_subtree: RouteSelection,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectionError {
    InvalidRoutePath(RoutePath),
    InvalidFusionDepth(u32),
    InvalidChoice(usize),
    InvalidMaterialIndex(usize),
    IncompatibleMaterial(DemonId),
    NoRoute,
}

impl RouteSelector {
    pub(crate) fn new(
        game_data: &GameData,
        player_context: &PlayerContext,
        routes: Vec<Rc<RouteSpace>>,
    ) -> Self {
        let route_count = routes.iter().fold(BigUint::default(), |total, space| {
            total + &space.route_count
        });
        let default_selection = routes
            .iter()
            .filter_map(|space| space.best_score().map(|score| (space, score)))
            .min_by(|(_, left), (_, right)| left.cmp(right))
            .map(|(space, _)| {
                Rc::clone(space)
                    .select_default(game_data, player_context)
                    .expect("ranked default must be selectable")
            });
        Self {
            routes,
            default_selection,
            route_count,
        }
    }

    pub fn route_space(&self, fusion_depth: u32) -> Option<&Rc<RouteSpace>> {
        self.routes.get(usize::try_from(fusion_depth).ok()?)
    }
}

impl RouteSpace {
    pub(crate) fn new(
        game_data: &GameData,
        demon: DemonId,
        required_skills: Vec<SkillId>,
        fusion_depth: u32,
        choices: Vec<RouteChoice>,
    ) -> Rc<Self> {
        let route_count = choices
            .iter()
            .map(|choice| match choice {
                RouteChoice::Direct(_) => BigUint::from(1_u8),
                RouteChoice::Fusion(fusion) => Self::count_fusion(fusion_depth, fusion),
            })
            .sum();
        let mut space = Self {
            demon,
            required_skills,
            fusion_depth,
            choices,
            route_count,
            best_choice: None,
        };
        space.best_choice = route_ranking::best_for_space(&space, game_data);
        debug_assert_eq!(
            space.best_choice.is_none(),
            space.route_count == BigUint::default()
        );
        Rc::new(space)
    }

    pub fn best_score(&self) -> Option<&RouteScore> {
        self.best_choice.as_ref().map(|choice| &choice.score)
    }

    pub fn choice_score(&self, game_data: &GameData, choice_index: usize) -> Option<RouteScore> {
        if let Some(best) = &self.best_choice
            && best.choice_index == choice_index
        {
            return Some(best.score.clone());
        }
        route_ranking::score_choice(self, game_data, choice_index)
    }

    pub fn choice(&self, choice_index: usize) -> Option<&RouteChoice> {
        self.choices.get(choice_index)
    }

    fn count_fusion(fusion_depth: u32, fusion: &FusionChoice) -> BigUint {
        let required_child_depth = fusion_depth - 1;
        let all = fusion
            .materials
            .iter()
            .map(|material| {
                material
                    .routes
                    .iter()
                    .fold(BigUint::default(), |total, child| {
                        total + &child.route_count
                    })
            })
            .product::<BigUint>();
        let shallower = fusion
            .materials
            .iter()
            .map(|material| {
                material
                    .routes
                    .iter()
                    .filter(|child| child.fusion_depth < required_child_depth)
                    .fold(BigUint::default(), |total, child| {
                        total + &child.route_count
                    })
            })
            .product::<BigUint>();
        all - shallower
    }

    fn select_default(
        self: Rc<Self>,
        game_data: &GameData,
        player_context: &PlayerContext,
    ) -> Result<RouteSelection, SelectionError> {
        let choice_index = self
            .best_choice
            .as_ref()
            .ok_or(SelectionError::NoRoute)?
            .choice_index;
        self.select(game_data, player_context, choice_index)
    }

    fn select(
        self: Rc<Self>,
        game_data: &GameData,
        player_context: &PlayerContext,
        choice_index: usize,
    ) -> Result<RouteSelection, SelectionError> {
        let choice = self
            .choice(choice_index)
            .ok_or(SelectionError::InvalidChoice(choice_index))?;
        if matches!(choice, RouteChoice::Direct(_)) != (self.fusion_depth == 0) {
            return Err(SelectionError::InvalidFusionDepth(self.fusion_depth));
        }
        let material_route_indices = route_ranking::material_route_indices(&self, choice_index)
            .ok_or(SelectionError::NoRoute)?;
        let materials = match choice {
            RouteChoice::Direct(_) => Vec::new(),
            RouteChoice::Fusion(fusion) => fusion
                .materials
                .iter()
                .zip(&material_route_indices)
                .map(|(material, &index)| {
                    Rc::clone(&material.routes[index]).select_default(game_data, player_context)
                })
                .collect::<Result<Vec<_>, _>>()?,
        };
        let route = self.materialize(game_data, choice_index, &materials)?;
        let request = SearchRequest {
            target: self.demon,
            required_skills: self.required_skills.clone(),
            max_fusion_depth: self.fusion_depth,
        };
        let demon = route_replay::replay(game_data, player_context, &request, route.as_ref())
            .expect("route-space selection must replay");
        Ok(RouteSelection {
            space: self,
            choice_index,
            demon,
            materials,
        })
    }

    fn materialize(
        &self,
        game_data: &GameData,
        choice_index: usize,
        materials: &[RouteSelection],
    ) -> Result<Rc<Route>, SelectionError> {
        match self
            .choice(choice_index)
            .ok_or(SelectionError::InvalidChoice(choice_index))?
        {
            RouteChoice::Direct(direct) => Ok(self.add_upgrades(
                game_data,
                direct.target_level,
                Rc::new(Route::Direct { demon: self.demon }),
            )),
            RouteChoice::Fusion(fusion) => {
                if fusion.materials.len() != materials.len() {
                    return Err(SelectionError::NoRoute);
                }
                let subroutes = fusion
                    .materials
                    .iter()
                    .zip(materials)
                    .map(|(material, child)| {
                        Ok(FusionSubroute {
                            required_skills: material.required_skills.clone(),
                            route: child.materialize_route(game_data)?,
                        })
                    })
                    .collect::<Result<Vec<_>, SelectionError>>()?;
                Ok(self.add_upgrades(
                    game_data,
                    fusion.target_level,
                    Rc::new(Route::Fusion {
                        recipe: fusion.recipe.as_ref().clone(),
                        materials: subroutes,
                    }),
                ))
            }
        }
    }

    fn add_upgrades(
        &self,
        game_data: &GameData,
        target_level: u32,
        mut route: Rc<Route>,
    ) -> Rc<Route> {
        let base_level = game_data
            .demons()
            .get(self.demon)
            .expect("route-space demon must exist")
            .base_level;
        for level in base_level + 1..=target_level {
            route = Rc::new(Route::Upgrade {
                level,
                previous: route,
            });
        }
        route
    }
}

impl RouteSelection {
    pub fn score(&self, _game_data: &GameData) -> RouteScore {
        route_ranking::actual_score(self)
    }

    pub fn materialize_route(&self, game_data: &GameData) -> Result<Rc<Route>, SelectionError> {
        self.space
            .materialize(game_data, self.choice_index, &self.materials)
    }

    pub fn apply_update(&mut self, update: RouteUpdate) -> Result<(), SelectionError> {
        Self::replace_at(self, &update.replace_from.0, update.new_subtree)
    }

    pub fn select_choice(
        &self,
        game_data: &GameData,
        player_context: &PlayerContext,
        path: &RoutePath,
        space: Rc<RouteSpace>,
        choice_index: usize,
    ) -> Result<RouteUpdate, SelectionError> {
        let selected = self
            .at(path)
            .ok_or_else(|| SelectionError::InvalidRoutePath(path.clone()))?;
        if selected.space.demon != space.demon
            || selected.space.required_skills != space.required_skills
        {
            return Err(SelectionError::NoRoute);
        }
        Ok(RouteUpdate {
            replace_from: path.clone(),
            new_subtree: space.select(game_data, player_context, choice_index)?,
        })
    }

    pub fn material_replacements(
        &self,
        parent: &RoutePath,
        material_index: usize,
    ) -> Result<Vec<DemonId>, SelectionError> {
        let (selected, _) = self.fusion_at(parent, material_index)?;
        let mut replacements = Vec::new();
        for choice in &selected.space.choices {
            let RouteChoice::Fusion(fusion) = choice else {
                continue;
            };
            for &material in &fusion.recipe.materials {
                if !replacements.contains(&material) {
                    replacements.push(material);
                }
            }
        }
        Ok(replacements)
    }

    pub fn replace_material(
        &self,
        game_data: &GameData,
        player_context: &PlayerContext,
        parent: &RoutePath,
        material_index: usize,
        replacement: DemonId,
    ) -> Result<RouteUpdate, SelectionError> {
        let (selected, _) = self.fusion_at(parent, material_index)?;
        let choice_index = selected
            .space
            .choices
            .iter()
            .enumerate()
            .filter(|(_, choice)| {
                matches!(choice, RouteChoice::Fusion(fusion) if fusion.recipe.materials.contains(&replacement))
            })
            .filter_map(|(index, _)| selected.space.choice_score(game_data, index).map(|score| (index, score)))
            .min_by(|(_, left), (_, right)| left.cmp(right))
            .map(|(index, _)| index)
            .ok_or(SelectionError::IncompatibleMaterial(replacement))?;
        Ok(RouteUpdate {
            replace_from: parent.clone(),
            new_subtree: Rc::clone(&selected.space).select(
                game_data,
                player_context,
                choice_index,
            )?,
        })
    }

    fn replace_at(
        selected: &mut Self,
        path: &[usize],
        replacement: RouteSelection,
    ) -> Result<(), SelectionError> {
        let Some((&material_index, remaining)) = path.split_first() else {
            if selected.space.demon != replacement.space.demon
                || selected.space.required_skills != replacement.space.required_skills
            {
                return Err(SelectionError::NoRoute);
            }
            *selected = replacement;
            return Ok(());
        };
        let material = selected
            .materials
            .get_mut(material_index)
            .ok_or_else(|| SelectionError::InvalidRoutePath(RoutePath(path.to_vec())))?;
        Self::replace_at(material, remaining, replacement)
    }

    fn at(&self, path: &RoutePath) -> Option<&Self> {
        let mut current = self;
        for &material_index in &path.0 {
            current = current.materials.get(material_index)?;
        }
        Some(current)
    }

    fn fusion_at<'a>(
        &'a self,
        path: &RoutePath,
        material_index: usize,
    ) -> Result<(&'a Self, &'a FusionChoice), SelectionError> {
        let selected = self
            .at(path)
            .ok_or_else(|| SelectionError::InvalidRoutePath(path.clone()))?;
        let fusion = match selected
            .space
            .choice(selected.choice_index)
            .ok_or(SelectionError::InvalidChoice(selected.choice_index))?
        {
            RouteChoice::Fusion(fusion) => fusion,
            RouteChoice::Direct(_) => {
                return Err(SelectionError::InvalidMaterialIndex(material_index));
            }
        };
        if material_index >= fusion.materials.len() {
            return Err(SelectionError::InvalidMaterialIndex(material_index));
        }
        Ok((selected, fusion))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn selection(demon: u32, required_skills: Vec<SkillId>) -> RouteSelection {
        RouteSelection {
            space: RouteSpace::new(
                &crate::dataset::game_data(),
                DemonId(demon),
                required_skills,
                0,
                Vec::new(),
            ),
            choice_index: 0,
            demon: Demon {
                meta: DemonId(demon),
                level: demon,
                skills: Vec::new(),
            },
            materials: Vec::new(),
        }
    }

    #[test]
    fn applies_compatible_subtree_updates_and_rejects_invalid_ones() {
        let mut root = selection(1, Vec::new());
        root.materials.push(selection(2, vec![SkillId(3)]));
        let mut replacement = selection(2, vec![SkillId(3)]);
        replacement.demon.level = 99;

        root.apply_update(RouteUpdate {
            replace_from: RoutePath(vec![0]),
            new_subtree: replacement,
        })
        .unwrap();
        assert_eq!(root.materials[0].demon.level, 99);

        let error = root
            .apply_update(RouteUpdate {
                replace_from: RoutePath(vec![0]),
                new_subtree: selection(4, vec![SkillId(3)]),
            })
            .unwrap_err();
        assert_eq!(error, SelectionError::NoRoute);
        assert_eq!(root.materials[0].demon.level, 99);

        let error = root
            .apply_update(RouteUpdate {
                replace_from: RoutePath(vec![1]),
                new_subtree: selection(2, vec![SkillId(3)]),
            })
            .unwrap_err();
        assert_eq!(error, SelectionError::InvalidRoutePath(RoutePath(vec![1])));
    }
}
