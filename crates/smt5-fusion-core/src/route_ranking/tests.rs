use std::rc::Rc;

use super::*;
use crate::{
    data::{
        demon_catalog::DemonCatalog, skill_catalog::SkillCatalog,
        special_recipe_catalog::SpecialRecipeCatalog,
    },
    model::{
        demon::{DemonContent, DemonId, DemonMeta},
        player_context::PlayerContext,
        race::Race,
        recipe::RecipeMeta,
        route_space::{DirectChoice, FusionChoice, FusionMaterialOptions, RouteSelector},
        skill::SkillId,
    },
};

fn data(prices: &[u32], recipes: Vec<RecipeMeta>) -> GameData {
    GameData::new(
        DemonCatalog::new(
            prices
                .iter()
                .enumerate()
                .map(|(id, &price)| DemonMeta {
                    id: DemonId(id as u32),
                    race: Race::Fiend,
                    base_level: 1,
                    content: DemonContent::Base,
                    compendium_price: price,
                    natural_skills: Vec::new(),
                    innate_skill: SkillId(0),
                })
                .collect(),
        ),
        SkillCatalog::new(Vec::new()),
        SpecialRecipeCatalog::new(recipes),
    )
}

fn recipe(result: u32, materials: &[u32]) -> RecipeMeta {
    RecipeMeta {
        result: DemonId(result),
        materials: materials.iter().copied().map(DemonId).collect(),
        is_special: true,
    }
}

fn leaf(data: &GameData, id: u32) -> Rc<RouteSpace> {
    RouteSpace::new(
        data,
        DemonId(id),
        Vec::new(),
        0,
        vec![RouteChoice::Direct(DirectChoice {
            target_level: 1,
            estimated_macca: data
                .demons()
                .get(DemonId(id))
                .unwrap()
                .compendium_price
                .into(),
        })],
    )
}

fn fusion_choice(id: u32, materials: Vec<Vec<Rc<RouteSpace>>>) -> RouteChoice {
    let material_ids = materials
        .iter()
        .map(|routes| routes[0].demon.0)
        .collect::<Vec<_>>();
    RouteChoice::Fusion(FusionChoice {
        target_level: 1,
        recipe: Rc::new(recipe(id, &material_ids)),
        materials: materials
            .into_iter()
            .map(|routes| FusionMaterialOptions {
                required_skills: Vec::new(),
                routes: routes.into(),
            })
            .collect(),
    })
}

fn fusion(
    data: &GameData,
    id: u32,
    depth: u32,
    materials: Vec<Vec<Rc<RouteSpace>>>,
) -> Rc<RouteSpace> {
    RouteSpace::new(
        data,
        DemonId(id),
        Vec::new(),
        depth,
        vec![fusion_choice(id, materials)],
    )
}

fn balanced(data: &GameData, id: u32, base: &Rc<RouteSpace>, depth: u32) -> Rc<RouteSpace> {
    if depth == 0 {
        return Rc::clone(base);
    }
    let child = balanced(data, id, base, depth - 1);
    fusion(data, id, depth, vec![vec![Rc::clone(&child)], vec![child]])
}

fn chain(data: &GameData, id: u32, base: &Rc<RouteSpace>, depth: u32) -> Rc<RouteSpace> {
    let mut current = Rc::clone(base);
    for depth in 1..=depth {
        current = fusion(data, id, depth, vec![vec![current], vec![Rc::clone(base)]]);
    }
    current
}

#[test]
fn default_prefers_four_fusions_at_depth_four_over_seven_at_depth_three() {
    let data = data(&[10], vec![recipe(0, &[0, 0])]);
    let mut context = PlayerContext::default();
    context.prepare_direct_recipes(&data);
    let base = leaf(&data, 0);
    let shallow = balanced(&data, 0, &base, 3);
    let deep = chain(&data, 0, &base, 4);
    assert_eq!(shallow.best_score().unwrap().fusion_count, 7_u8.into());
    let mut spaces = (0..3)
        .map(|depth| RouteSpace::new(&data, DemonId(0), Vec::new(), depth, Vec::new()))
        .collect::<Vec<_>>();
    spaces.extend([shallow, deep]);
    let selector = RouteSelector::new(&data, &context, spaces);
    let selected = selector.default_selection.unwrap();
    assert_eq!(
        selected.score(&data),
        RouteScore {
            fusion_count: 4_u8.into(),
            fusion_depth: 4,
            estimated_macca: 50_u8.into()
        }
    );
    assert_eq!(selector.route_count, 2_u8.into());
}

#[test]
fn score_priorities_are_fusion_count_then_depth_then_macca() {
    let score = |count: u32, depth, macca: u32| RouteScore {
        fusion_count: count.into(),
        fusion_depth: depth,
        estimated_macca: macca.into(),
    };
    assert!(score(3, 3, 1_000_000) < score(4, 2, 1));
    assert!(score(3, 2, 1_000_000) < score(3, 3, 1));
    assert!(score(3, 2, 1) < score(3, 2, 2));
}

#[test]
fn upgrades_are_free_and_do_not_break_ties() {
    let data = data(&[123], Vec::new());
    let space = RouteSpace::new(
        &data,
        DemonId(0),
        Vec::new(),
        0,
        vec![
            RouteChoice::Direct(DirectChoice {
                target_level: 99,
                estimated_macca: 123_u8.into(),
            }),
            RouteChoice::Direct(DirectChoice {
                target_level: 1,
                estimated_macca: 123_u8.into(),
            }),
        ],
    );
    assert_eq!(space.choice_score(&data, 0), space.choice_score(&data, 1));
    let best = best_for_space(&space, &data).unwrap();
    assert_eq!(best.choice_index, 0);
    assert_eq!(best.score.estimated_macca, 123_u8.into());
}

#[test]
fn equal_fusion_counts_and_depths_choose_cheaper_leaves_even_with_upgrades() {
    let data = data(&[10_000, 1, 100], Vec::new());
    let cheap = leaf(&data, 1);
    let expensive = leaf(&data, 2);
    let mut cheap_choice = fusion_choice(0, vec![vec![Rc::clone(&cheap)], vec![cheap]]);
    let RouteChoice::Fusion(fusion) = &mut cheap_choice else {
        unreachable!()
    };
    fusion.target_level = 99;
    let space = RouteSpace::new(
        &data,
        DemonId(0),
        Vec::new(),
        1,
        vec![
            fusion_choice(0, vec![vec![Rc::clone(&expensive)], vec![expensive]]),
            cheap_choice,
        ],
    );
    let best = best_for_space(&space, &data).unwrap();
    assert_eq!(best.choice_index, 1);
    assert_eq!(best.score.estimated_macca, 2_u8.into());
}

#[test]
fn exact_depth_is_reached_by_the_cheapest_material_not_the_last_one() {
    let data = data(&[10, 1, 10, 100, 1_000], Vec::new());
    let cheap = leaf(&data, 1);
    let expensive = leaf(&data, 3);
    let first_deep = fusion(&data, 0, 1, vec![vec![Rc::clone(&cheap)], vec![cheap]]);
    let last_deep = fusion(
        &data,
        2,
        1,
        vec![vec![Rc::clone(&expensive)], vec![expensive]],
    );
    let space = fusion(
        &data,
        4,
        2,
        vec![
            vec![leaf(&data, 0), first_deep],
            vec![leaf(&data, 2), last_deep],
        ],
    );
    let best = best_for_space(&space, &data).unwrap();
    assert_eq!(
        material_route_indices(&space, best.choice_index),
        Some(vec![1, 0])
    );
    assert_eq!(
        best.score,
        RouteScore {
            fusion_count: 2_u8.into(),
            fusion_depth: 2,
            estimated_macca: 12_u8.into()
        }
    );
}

#[test]
fn a_branch_can_be_deeper_and_cheaper_when_another_branch_fixes_the_depth() {
    let data = data(&[1_000, 1, 100, 1_000], Vec::new());
    let shallow = balanced(&data, 0, &leaf(&data, 2), 2);
    let deep = chain(&data, 0, &leaf(&data, 1), 3);
    assert_eq!(
        shallow.best_score().unwrap().fusion_count,
        deep.best_score().unwrap().fusion_count
    );
    let space = fusion(
        &data,
        3,
        4,
        vec![vec![shallow, Rc::clone(&deep)], vec![deep]],
    );
    let best = best_for_space(&space, &data).unwrap();
    assert_eq!(
        material_route_indices(&space, best.choice_index),
        Some(vec![1, 0])
    );
    assert_eq!(
        best.score,
        RouteScore {
            fusion_count: 7_u8.into(),
            fusion_depth: 4,
            estimated_macca: 8_u8.into()
        }
    );
}

#[test]
fn tied_material_combinations_keep_stable_route_indices() {
    let data = data(&[10], Vec::new());
    let base = leaf(&data, 0);
    let deep = balanced(&data, 0, &base, 1);
    let space = fusion(
        &data,
        0,
        2,
        vec![vec![Rc::clone(&base), Rc::clone(&deep)], vec![base, deep]],
    );
    assert_eq!(
        material_route_indices(&space, best_for_space(&space, &data).unwrap().choice_index),
        Some(vec![0, 1])
    );
}

#[test]
fn special_fusions_count_once_and_shared_leaves_are_charged_per_occurrence() {
    let data = data(&[11], vec![recipe(0, &[0, 0, 0, 0])]);
    let mut context = PlayerContext::default();
    context.prepare_direct_recipes(&data);
    let base = leaf(&data, 0);
    let space = fusion(&data, 0, 1, vec![vec![Rc::clone(&base)]; 4]);
    let expected = RouteScore {
        fusion_count: 1_u8.into(),
        fusion_depth: 1,
        estimated_macca: 44_u8.into(),
    };
    assert_eq!(space.best_score(), Some(&expected));
    let selector = RouteSelector::new(&data, &context, vec![space]);
    assert_eq!(selector.default_selection.unwrap().score(&data), expected);
}

#[test]
fn scores_remain_exact_beyond_machine_integer_limits_without_expanding_routes() {
    let data = data(&[u32::MAX], Vec::new());
    let space = balanced(&data, 0, &leaf(&data, 0), 80);
    let score = space.best_score().unwrap();
    assert_eq!(score.fusion_count, (BigUint::from(1_u8) << 80_usize) - 1_u8);
    assert_eq!(score.estimated_macca, BigUint::from(u32::MAX) << 80_usize);
    assert_eq!(space.route_count, 1_u8.into());
}

#[test]
fn empty_spaces_have_no_best_choice() {
    let data = data(&[1], Vec::new());
    let space = RouteSpace::new(&data, DemonId(0), Vec::new(), 2, Vec::new());
    assert!(space.best_score().is_none());
    assert!(space.choice_score(&data, 0).is_none());
    assert!(material_route_indices(&space, 0).is_none());
}

#[test]
fn scoring_and_reconstruction_match_exhaustive_material_choices() {
    fn enumerate(
        fusion: &FusionChoice,
        exact_depth: u32,
        indices: &mut Vec<usize>,
        score: RouteScore,
        best: &mut Option<(RouteScore, Vec<usize>)>,
    ) {
        let Some(material) = fusion.materials.get(indices.len()) else {
            if score.fusion_depth == exact_depth {
                let candidate = (score, indices.clone());
                if best.as_ref().is_none_or(|best| &candidate < best) {
                    *best = Some(candidate);
                }
            }
            return;
        };
        for (index, child) in material.routes.iter().enumerate() {
            let Some(child_score) = child.best_score() else {
                continue;
            };
            indices.push(index);
            enumerate(
                fusion,
                exact_depth,
                indices,
                RouteScore {
                    fusion_count: &score.fusion_count + &child_score.fusion_count,
                    fusion_depth: score.fusion_depth.max(1 + child_score.fusion_depth),
                    estimated_macca: &score.estimated_macca + &child_score.estimated_macca,
                },
                best,
            );
            indices.pop();
        }
    }

    let data = data(&[20, 1], Vec::new());
    for depth in 1..=4 {
        for material_count in 2..=4 {
            for mask in 0..(1 << depth) {
                let materials = (0..material_count)
                    .map(|index| {
                        (0..depth)
                            .map(|child_depth| {
                                if mask & (1 << child_depth) == 0 {
                                    RouteSpace::new(
                                        &data,
                                        DemonId(0),
                                        Vec::new(),
                                        child_depth,
                                        Vec::new(),
                                    )
                                } else if child_depth == 0 {
                                    leaf(&data, 0)
                                } else {
                                    chain(&data, 0, &leaf(&data, index % 2), child_depth)
                                }
                            })
                            .collect()
                    })
                    .collect();
                let space = fusion(&data, 0, depth, materials);
                let RouteChoice::Fusion(fusion) = space.choice(0).unwrap() else {
                    unreachable!()
                };
                let mut expected = None;
                enumerate(
                    fusion,
                    depth,
                    &mut Vec::new(),
                    RouteScore {
                        fusion_count: 1_u8.into(),
                        fusion_depth: 1,
                        estimated_macca: BigUint::default(),
                    },
                    &mut expected,
                );
                let actual = score_choice(&space, &data, 0).zip(material_route_indices(&space, 0));
                assert_eq!(
                    actual, expected,
                    "depth={depth}, materials={material_count}, mask={mask}"
                );
                assert_eq!(space.best_score(), actual.as_ref().map(|(score, _)| score));
            }
        }
    }
}
