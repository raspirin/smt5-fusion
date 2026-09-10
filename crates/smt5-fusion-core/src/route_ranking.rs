use num_bigint::BigUint;

use crate::{
    data::game_data::GameData,
    model::route_space::{FusionChoice, RouteChoice, RouteSelection, RouteSpace},
};

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct RouteScore {
    pub fusion_count: BigUint,
    pub fusion_depth: u32,
    pub estimated_macca: BigUint,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BestChoice {
    pub score: RouteScore,
    pub choice_index: usize,
}

#[derive(Default, PartialEq, Eq, PartialOrd, Ord)]
struct MaterialCombination<T> {
    fusion_count: BigUint,
    estimated_macca: BigUint,
    route_indices: T,
}

trait MaterialTrace: Default + Ord {
    fn with_route(&self, index: usize) -> Self;
}

impl MaterialTrace for () {
    fn with_route(&self, _: usize) -> Self {}
}

impl MaterialTrace for Vec<usize> {
    fn with_route(&self, index: usize) -> Self {
        let mut indices = self.clone();
        indices.push(index);
        indices
    }
}

pub(crate) fn best_for_space(space: &RouteSpace, game_data: &GameData) -> Option<BestChoice> {
    (0..space.choices.len())
        .filter_map(|choice_index| {
            score_choice(space, game_data, choice_index).map(|score| BestChoice {
                score,
                choice_index,
            })
        })
        .min_by(|left, right| left.score.cmp(&right.score))
}

pub(crate) fn score_choice(
    space: &RouteSpace,
    game_data: &GameData,
    choice_index: usize,
) -> Option<RouteScore> {
    let choice = space.choice(choice_index)?;
    let RouteChoice::Fusion(fusion) = choice else {
        return (space.fusion_depth == 0).then(|| RouteScore {
            fusion_count: BigUint::default(),
            fusion_depth: 0,
            estimated_macca: game_data
                .demons()
                .get(space.demon)
                .expect("route-space demon must exist")
                .compendium_price
                .into(),
        });
    };
    let required_depth = space.fusion_depth.checked_sub(1)?;
    let best = best_material_combination::<()>(fusion, required_depth)?;
    Some(RouteScore {
        fusion_count: best.fusion_count + 1_u8,
        fusion_depth: space.fusion_depth,
        estimated_macca: best.estimated_macca,
    })
}

pub(crate) fn material_route_indices(
    space: &RouteSpace,
    choice_index: usize,
) -> Option<Vec<usize>> {
    match space.choice(choice_index)? {
        RouteChoice::Direct(_) => (space.fusion_depth == 0).then(Vec::new),
        RouteChoice::Fusion(fusion) => {
            let required_depth = space.fusion_depth.checked_sub(1)?;
            best_material_combination::<Vec<usize>>(fusion, required_depth)
                .map(|best| best.route_indices)
        }
    }
}

fn best_material_combination<T: MaterialTrace>(
    fusion: &FusionChoice,
    required_depth: u32,
) -> Option<MaterialCombination<T>> {
    let mut memo = [Some(MaterialCombination::<T>::default()), None];
    for material in &fusion.materials {
        let mut new_memo: [Option<MaterialCombination<T>>; 2] = [None, None];
        for (reached, partial) in memo.iter().enumerate() {
            let Some(partial) = partial else { continue };
            for (index, child) in material.routes.iter().enumerate() {
                if child.fusion_depth > required_depth {
                    continue;
                }
                let Some(score) = child.best_score() else {
                    continue;
                };
                let now_reached = reached | usize::from(child.fusion_depth == required_depth);
                let fusion_count = &partial.fusion_count + &score.fusion_count;
                let estimated_macca = &partial.estimated_macca + &score.estimated_macca;
                let incumbent = &new_memo[now_reached];
                if incumbent.as_ref().is_some_and(|best| {
                    (&fusion_count, &estimated_macca) > (&best.fusion_count, &best.estimated_macca)
                }) {
                    continue;
                }
                let candidate = MaterialCombination {
                    fusion_count,
                    estimated_macca,
                    route_indices: partial.route_indices.with_route(index),
                };
                if incumbent.as_ref().is_none_or(|best| &candidate < best) {
                    new_memo[now_reached] = Some(candidate);
                }
            }
        }
        memo = new_memo;
    }
    memo[1].take()
}

pub(crate) fn actual_score(selection: &RouteSelection, game_data: &GameData) -> RouteScore {
    match selection
        .space
        .choice(selection.choice_index)
        .expect("selection choice must exist")
    {
        RouteChoice::Direct(_) => RouteScore {
            estimated_macca: game_data
                .demons()
                .get(selection.space.demon)
                .expect("selection demon must exist")
                .compendium_price
                .into(),
            ..RouteScore::default()
        },
        RouteChoice::Fusion(_) => {
            let mut score = RouteScore {
                fusion_count: 1_u8.into(),
                fusion_depth: 1,
                estimated_macca: BigUint::default(),
            };
            for material in &selection.materials {
                let child = actual_score(material, game_data);
                score.fusion_count += child.fusion_count;
                score.fusion_depth = score.fusion_depth.max(1 + child.fusion_depth);
                score.estimated_macca += child.estimated_macca;
            }
            score
        }
    }
}

#[cfg(test)]
mod tests;
