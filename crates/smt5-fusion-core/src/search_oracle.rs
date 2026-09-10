use std::{collections::HashMap, rc::Rc};

use num_bigint::BigUint;

use crate::{
    data::{
        demon_catalog::DemonCatalog, game_data::GameData, skill_catalog::SkillCatalog,
        special_recipe_catalog::SpecialRecipeCatalog,
    },
    model::{
        demon::{DemonContent, DemonId, DemonMeta, NaturalSkill, SkillAcquisition},
        player_context::PlayerContext,
        race::Race,
        recipe::RecipeMeta,
        route::{FusionSubroute, Route},
        route_space::{RouteChoice, RouteSelector, RouteSpace},
        skill::{Skill, SkillCategory, SkillFlags, SkillId},
    },
    reverse_search::{SearchError, SearchRequest, SearchSolution, search},
    route_replay::{ReplayError, replay},
};

const QUERY_SKILLS: [SkillId; 4] = [SkillId(1), SkillId(2), SkillId(3), SkillId(4)];
const MASK_COUNT: usize = 1 << QUERY_SKILLS.len();
const MAX_DEPTH: u32 = 3;

type SkillMask = u8;
type OracleKey = (DemonId, SkillMask, u32);

fn search_request(
    target: DemonId,
    required_skills: Vec<SkillId>,
    max_fusion_depth: u32,
) -> SearchRequest {
    SearchRequest {
        target,
        required_skills,
        max_fusion_depth,
    }
}

fn search_solutions(
    game_data: &GameData,
    player_context: &PlayerContext,
    request: &SearchRequest,
) -> Result<Vec<SearchSolution>, SearchError> {
    search(game_data, player_context, request)
        .map(|selector| expand_selector(game_data, player_context, request, &selector))
}

fn expand_selector(
    game_data: &GameData,
    player_context: &PlayerContext,
    request: &SearchRequest,
    selector: &RouteSelector,
) -> Vec<SearchSolution> {
    selector
        .routes
        .iter()
        .flat_map(|space| {
            expand_space(game_data, space)
                .into_iter()
                .map(|route| SearchSolution {
                    demon: replay(game_data, player_context, request, route.as_ref()).unwrap(),
                    fusion_depth: space.fusion_depth,
                    route,
                })
        })
        .collect()
}

fn expand_space(game_data: &GameData, space: &Rc<RouteSpace>) -> Vec<Rc<Route>> {
    let base_level = game_data.demons().get(space.demon).unwrap().base_level;
    let mut result = Vec::new();
    for choice in &space.choices {
        match choice {
            RouteChoice::Direct(direct) => result.push(Rc::new(add_upgrades(
                base_level,
                direct.target_level,
                Route::Direct { demon: space.demon },
            ))),
            RouteChoice::Fusion(fusion) => {
                let options = fusion
                    .materials
                    .iter()
                    .map(|material| {
                        material
                            .routes
                            .iter()
                            .flat_map(|child| {
                                expand_space(game_data, child)
                                    .into_iter()
                                    .map(move |route| (child.fusion_depth, route))
                            })
                            .collect::<Vec<_>>()
                    })
                    .collect::<Vec<_>>();
                if options.iter().any(Vec::is_empty) {
                    continue;
                }
                combine(&options, 0, &mut Vec::new(), &mut |selected| {
                    if selected.iter().map(|(depth, _)| *depth).max().unwrap() + 1
                        != space.fusion_depth
                    {
                        return;
                    }
                    let materials = fusion
                        .materials
                        .iter()
                        .zip(selected)
                        .map(|(material, (_, route))| FusionSubroute {
                            required_skills: material.required_skills.clone(),
                            route: Rc::clone(route),
                        })
                        .collect();
                    result.push(Rc::new(add_upgrades(
                        base_level,
                        fusion.target_level,
                        Route::Fusion {
                            recipe: fusion.recipe.as_ref().clone(),
                            materials,
                        },
                    )));
                });
            }
        }
    }
    result
}

#[test]
fn every_search_result_matches_the_bottom_up_oracle() {
    let data = game_data();
    let mut context = PlayerContext::default();
    context.prepare_direct_recipes(&data);
    let oracle = build_oracle(&data, &context);

    for target in data.demons().iter().map(|demon| demon.id) {
        for required_skills in 0..MASK_COUNT as SkillMask {
            let skill_ids = skill_ids(required_skills);
            for max_depth in 0..=MAX_DEPTH {
                let request = search_request(target, skill_ids.clone(), max_depth);
                let expected = (0..=max_depth)
                    .flat_map(|depth| {
                        oracle
                            .get(&(target, required_skills, depth))
                            .unwrap()
                            .iter()
                            .cloned()
                    })
                    .collect::<Vec<_>>();
                let selector = search(&data, &context, &request).unwrap();
                let solutions = expand_selector(&data, &context, &request, &selector);
                assert_eq!(
                    selector.route_count,
                    BigUint::from(expected.len()),
                    "request={request:?}"
                );
                let actual = solutions
                    .iter()
                    .map(|solution| solution.route.as_ref().clone())
                    .collect::<Vec<_>>();
                assert_same_routes(&actual, &expected, &request);
                let expected_score = expected.iter().map(|route| route_score(&data, route)).min();
                match (selector.default_selection.as_ref(), expected_score) {
                    (Some(selection), Some(expected_score)) => {
                        let route = selection.materialize_route(&data).unwrap();
                        assert!(expected.contains(route.as_ref()), "request={request:?}");
                        assert_eq!(
                            route_score(&data, &route),
                            expected_score,
                            "request={request:?}"
                        );
                    }
                    (None, None) => {}
                    _ => panic!("default-route mismatch for {request:?}"),
                }
                for (index, solution) in solutions.iter().enumerate() {
                    assert_eq!(solution.fusion_depth, route_depth(&solution.route));
                    assert_eq!(
                        solution.demon,
                        replay(&data, &context, &request, &solution.route).unwrap()
                    );
                    assert!(
                        actual[..index]
                            .iter()
                            .all(|previous| previous != solution.route.as_ref())
                    );
                }
            }
        }
    }
}

#[test]
fn fusion_remains_available_when_the_result_already_has_the_skill() {
    let data = game_data();
    let mut context = PlayerContext::default();
    context.prepare_direct_recipes(&data);
    let request = search_request(DemonId(6), vec![SkillId(4)], 2);
    let solutions = search_solutions(&data, &context, &request).unwrap();

    assert_eq!(
        solutions
            .iter()
            .map(|solution| solution.fusion_depth)
            .collect::<Vec<_>>(),
        vec![0, 1, 2]
    );
}

#[test]
fn search_is_stable_and_normalizes_required_skills() {
    let data = game_data();
    let mut context = PlayerContext::default();
    context.prepare_direct_recipes(&data);
    let canonical = search_request(DemonId(7), vec![SkillId(1), SkillId(2)], 2);
    let reordered = search_request(DemonId(7), vec![SkillId(2), SkillId(1), SkillId(2)], 2);

    let default = search(&data, &context, &canonical)
        .unwrap()
        .default_selection;
    assert_eq!(
        search(&data, &context, &canonical)
            .unwrap()
            .default_selection,
        default
    );
    assert_eq!(
        search(&data, &context, &reordered)
            .unwrap()
            .default_selection,
        default
    );
    let expected = search_solutions(&data, &context, &canonical).unwrap();
    assert_eq!(
        search_solutions(&data, &context, &canonical).unwrap(),
        expected
    );
    assert_eq!(
        search_solutions(&data, &context, &reordered).unwrap(),
        expected
    );
}

#[test]
fn route_selector_replaces_material_with_the_best_compatible_choice() {
    let data = crate::dataset::game_data();
    let mut context = PlayerContext::default();
    context.set_konohana_sakuya_dlc(true);
    context.set_dagda_dlc(true);
    context.prepare_direct_recipes(&data);
    let request = search_request(crate::dataset::demon_ids::PIXIE, Vec::new(), 1);
    let selector = search(&data, &context, &request).unwrap();
    let root = crate::model::route_space::RoutePath(Vec::new());
    let current = selector
        .default_selection
        .as_ref()
        .unwrap()
        .select_choice(&data, &context, &root, Rc::clone(&selector.routes[1]), 0)
        .unwrap()
        .new_subtree;
    let current_route = current.materialize_route(&data).unwrap();
    let Route::Fusion {
        recipe: current_recipe,
        ..
    } = current_route.as_ref()
    else {
        panic!("expected fusion route");
    };
    let replacement = current
        .material_replacements(&root, 0)
        .unwrap()
        .into_iter()
        .find(|material| !current_recipe.materials.contains(material))
        .expect("expected a material from another recipe");

    let update = current
        .replace_material(&data, &context, &root, 0, replacement)
        .unwrap();
    let route = update.new_subtree.materialize_route(&data).unwrap();
    let Route::Fusion { recipe, .. } = route.as_ref() else {
        panic!("expected fusion route");
    };
    assert!(recipe.materials.contains(&replacement));
    let best = expand_space(&data, &current.space).into_iter()
        .filter(|route| matches!(route.as_ref(), Route::Fusion { recipe, .. } if recipe.materials.contains(&replacement)))
        .map(|route| route_score(&data, &route))
        .min().unwrap();
    assert_eq!(route_score(&data, &route), best);
}

#[test]
fn replay_rejects_invalid_routes() {
    let data = game_data();
    let mut context = PlayerContext::default();
    context.prepare_direct_recipes(&data);
    let upgrade_request = search_request(DemonId(2), vec![SkillId(2)], 0);

    assert_eq!(
        replay(
            &data,
            &context,
            &upgrade_request,
            &Route::Direct { demon: DemonId(1) },
        ),
        Err(ReplayError::UnexpectedDemon {
            expected: DemonId(2),
            actual: DemonId(1),
        })
    );
    assert_eq!(
        replay(
            &data,
            &context,
            &upgrade_request,
            &Route::Direct { demon: DemonId(2) },
        ),
        Err(ReplayError::InvalidDirect(DemonId(2)))
    );
    assert_eq!(
        replay(
            &data,
            &context,
            &upgrade_request,
            &Route::Upgrade {
                level: 3,
                previous: Rc::new(Route::Direct { demon: DemonId(2) }),
            },
        ),
        Err(ReplayError::InvalidUpgrade {
            demon: DemonId(2),
            level: 3,
        })
    );

    let request = search_request(DemonId(5), vec![SkillId(1)], 1);
    let mut solutions = search_solutions(&data, &context, &request).unwrap();
    let fusion_route = solutions[0].route.clone();
    assert_eq!(
        replay(
            &data,
            &context,
            &search_request(DemonId(5), vec![SkillId(1)], 0),
            &fusion_route,
        ),
        Err(ReplayError::FusionDepthExceeded {
            actual: 1,
            maximum: 0,
        })
    );
    let Route::Fusion { materials, .. } = Rc::make_mut(&mut solutions[0].route) else {
        panic!("expected a fusion route");
    };
    materials
        .iter_mut()
        .for_each(|material| material.required_skills.clear());
    assert_eq!(
        replay(&data, &context, &request, &solutions[0].route),
        Err(ReplayError::InvalidFusion(DemonId(5)))
    );
}

#[test]
fn search_validates_requests() {
    let data = game_data();
    let mut context = PlayerContext::default();
    context.prepare_direct_recipes(&data);

    assert_eq!(
        search(&data, &context, &search_request(DemonId(99), Vec::new(), 0),),
        Err(SearchError::UnknownDemon(DemonId(99)))
    );
    assert_eq!(
        search(
            &data,
            &context,
            &search_request(DemonId(1), vec![SkillId(99)], 0),
        ),
        Err(SearchError::UnknownSkill(SkillId(99)))
    );
    assert_eq!(
        search(
            &data,
            &context,
            &search_request(DemonId(1), vec![SkillId(1); 9], 0),
        )
        .unwrap(),
        search(
            &data,
            &context,
            &search_request(DemonId(1), vec![SkillId(1)], 0),
        )
        .unwrap()
    );
    assert_eq!(
        search(
            &data,
            &context,
            &search_request(DemonId(1), (1..=9).map(SkillId).collect(), 0),
        ),
        Err(SearchError::TooManySkills {
            selected: 9,
            maximum: 8,
        })
    );
}

#[test]
fn formal_data_solutions_are_valid() {
    let data = crate::dataset::game_data();
    let mut context = PlayerContext::default();
    context.set_konohana_sakuya_dlc(true);
    context.set_dagda_dlc(true);
    context.prepare_direct_recipes(&data);
    let requests = [
        search_request(
            crate::dataset::demon_ids::ABADDON,
            vec![crate::dataset::skill_ids::MARAGIBARION],
            0,
        ),
        search_request(
            crate::dataset::demon_ids::ABADDON,
            vec![crate::dataset::skill_ids::AGI],
            1,
        ),
        search_request(
            crate::dataset::demon_ids::SATAN,
            vec![crate::dataset::skill_ids::HAMABARION],
            1,
        ),
    ];

    for request in requests {
        let solutions = search_solutions(&data, &context, &request).unwrap();
        assert!(!solutions.is_empty(), "expected solutions for {request:?}");
        if request.target == crate::dataset::demon_ids::SATAN {
            assert!(solutions.iter().any(|solution| {
                matches!(solution.route.as_ref(), Route::Fusion { recipe, .. } if recipe.is_special)
            }));
        }
        for (index, solution) in solutions.iter().enumerate() {
            assert_eq!(solution.fusion_depth, route_depth(&solution.route));
            assert!(
                solutions[..index]
                    .iter()
                    .all(|previous| previous.route != solution.route)
            );
            assert_eq!(
                solution.demon,
                replay(&data, &context, &request, &solution.route).unwrap()
            );
        }
    }
}

fn build_oracle(
    game_data: &GameData,
    player_context: &PlayerContext,
) -> HashMap<OracleKey, Vec<Route>> {
    let demons = game_data
        .demons()
        .iter()
        .map(|demon| demon.id)
        .collect::<Vec<_>>();
    let mut routes = HashMap::<OracleKey, Vec<Route>>::new();

    for &demon in &demons {
        for required_skills in 0..MASK_COUNT as SkillMask {
            let direct = direct_route(game_data.demons().get(demon).unwrap(), required_skills);
            routes.insert((demon, required_skills, 0), direct.into_iter().collect());
        }
    }

    for exact_depth in 1..=MAX_DEPTH {
        for &demon in &demons {
            for required_skills in 0..MASK_COUNT as SkillMask {
                let mut state_routes = Vec::new();
                let demon_meta = game_data.demons().get(demon).unwrap();
                for (target_level, local_skills) in target_levels(demon_meta, required_skills) {
                    let skills_to_inherit = required_skills & !local_skills;
                    if !all_inheritable(game_data, skills_to_inherit) {
                        continue;
                    }
                    for recipe in player_context.get_direct_recipes(demon).unwrap_or_default() {
                        for material_skills in
                            assignments(skills_to_inherit, recipe.materials.len())
                        {
                            let options = recipe
                                .materials
                                .iter()
                                .zip(&material_skills)
                                .map(|(material, required)| {
                                    (0..exact_depth)
                                        .flat_map(|depth| {
                                            routes
                                                .get(&(*material, *required, depth))
                                                .unwrap()
                                                .iter()
                                                .cloned()
                                                .map(move |route| (depth, route))
                                        })
                                        .collect::<Vec<_>>()
                                })
                                .collect::<Vec<_>>();
                            if options.iter().any(Vec::is_empty) {
                                continue;
                            }
                            let mut selected = Vec::new();
                            combine(&options, 0, &mut selected, &mut |combination| {
                                if combination.iter().map(|(depth, _)| *depth).max().unwrap() + 1
                                    != exact_depth
                                {
                                    return;
                                }
                                let materials = material_skills
                                    .iter()
                                    .copied()
                                    .zip(combination.iter().map(|(_, route)| route.clone()))
                                    .map(|(required, route)| FusionSubroute {
                                        required_skills: skill_ids(required),
                                        route: Rc::new(route),
                                    })
                                    .collect();
                                state_routes.push(add_upgrades(
                                    demon_meta.base_level,
                                    target_level,
                                    Route::Fusion {
                                        recipe: recipe.clone(),
                                        materials,
                                    },
                                ));
                            });
                        }
                    }
                }
                routes.insert((demon, required_skills, exact_depth), state_routes);
            }
        }
    }

    routes
}

fn direct_route(demon: &DemonMeta, required_skills: SkillMask) -> Option<Route> {
    target_levels(demon, required_skills)
        .into_iter()
        .find(|(_, local)| required_skills & !local == 0)
        .map(|(target_level, _)| {
            add_upgrades(
                demon.base_level,
                target_level,
                Route::Direct { demon: demon.id },
            )
        })
}

fn target_levels(demon: &DemonMeta, required_skills: SkillMask) -> Vec<(u32, SkillMask)> {
    let base_level = demon.base_level;
    let maximum_level = demon
        .natural_skills
        .iter()
        .filter_map(|natural_skill| match natural_skill.acquisition {
            SkillAcquisition::Initial { .. } => None,
            SkillAcquisition::Level { level } => Some(level),
        })
        .max()
        .unwrap_or(base_level);
    let mut result = Vec::new();

    for level in base_level..=maximum_level {
        let local_skills = natural_mask(demon, level) & required_skills;
        if result
            .last()
            .is_none_or(|(_, previous)| *previous != local_skills)
        {
            result.push((level, local_skills));
        }
    }
    result
}

fn natural_mask(demon: &DemonMeta, target_level: u32) -> SkillMask {
    demon.natural_skills.iter().fold(0, |mask, natural_skill| {
        let acquired = match natural_skill.acquisition {
            SkillAcquisition::Initial { .. } => true,
            SkillAcquisition::Level { level } => level <= target_level,
        };
        let bit = QUERY_SKILLS
            .iter()
            .position(|skill| *skill == natural_skill.skill)
            .map_or(0, |index| 1 << index);
        if acquired { mask | bit } else { mask }
    })
}

fn all_inheritable(game_data: &GameData, skills: SkillMask) -> bool {
    skill_ids(skills)
        .iter()
        .all(|skill| game_data.skills().get(*skill).unwrap().inheritable)
}

fn assignments(skills: SkillMask, material_count: usize) -> Vec<Vec<SkillMask>> {
    let skill_bits = (0..QUERY_SKILLS.len())
        .map(|index| 1 << index)
        .filter(|bit| skills & bit != 0)
        .collect::<Vec<_>>();
    let mut result = Vec::new();

    fn visit(
        skill_bits: &[SkillMask],
        index: usize,
        current: &mut [SkillMask],
        result: &mut Vec<Vec<SkillMask>>,
    ) {
        if index == skill_bits.len() {
            result.push(current.to_vec());
            return;
        }
        for material in 0..current.len() {
            current[material] |= skill_bits[index];
            visit(skill_bits, index + 1, current, result);
            current[material] &= !skill_bits[index];
        }
    }

    visit(&skill_bits, 0, &mut vec![0; material_count], &mut result);
    result
}

fn combine<T: Clone>(
    options: &[Vec<T>],
    index: usize,
    selected: &mut Vec<T>,
    visit: &mut impl FnMut(&[T]),
) {
    if index == options.len() {
        visit(selected);
        return;
    }
    for option in &options[index] {
        selected.push(option.clone());
        combine(options, index + 1, selected, visit);
        selected.pop();
    }
}

fn add_upgrades(base_level: u32, target_level: u32, mut route: Route) -> Route {
    for level in base_level + 1..=target_level {
        route = Route::Upgrade {
            level,
            previous: Rc::new(route),
        };
    }
    route
}

fn skill_ids(mask: SkillMask) -> Vec<SkillId> {
    QUERY_SKILLS
        .iter()
        .enumerate()
        .filter_map(|(index, skill)| (mask & (1 << index) != 0).then_some(*skill))
        .collect()
}

fn assert_same_routes(actual: &[Route], expected: &[Route], request: &SearchRequest) {
    assert_eq!(actual.len(), expected.len(), "request={request:?}");
    let mut unmatched = expected.to_vec();
    for route in actual {
        let index = unmatched
            .iter()
            .position(|expected| expected == route)
            .unwrap_or_else(|| panic!("unexpected route for {request:?}: {route:?}"));
        unmatched.remove(index);
    }
    assert!(unmatched.is_empty(), "request={request:?}");
}

fn route_score(data: &GameData, route: &Route) -> (u64, u32, u64) {
    match route {
        Route::Direct { demon } => (
            0,
            0,
            u64::from(data.demons().get(*demon).unwrap().compendium_price),
        ),
        Route::Upgrade { previous, .. } => route_score(data, previous),
        Route::Fusion { materials, .. } => {
            materials
                .iter()
                .fold((1, 1, 0), |(count, depth, cost), material| {
                    let child = route_score(data, &material.route);
                    (count + child.0, depth.max(1 + child.1), cost + child.2)
                })
        }
    }
}

fn route_depth(route: &Route) -> u32 {
    match route {
        Route::Direct { .. } => 0,
        Route::Upgrade { previous, .. } => route_depth(previous),
        Route::Fusion { materials, .. } => {
            1 + materials
                .iter()
                .map(|material| route_depth(&material.route))
                .max()
                .unwrap()
        }
    }
}

fn game_data() -> GameData {
    GameData::new(
        DemonCatalog::new(vec![
            demon(1, vec![initial(1, 1)]),
            demon(2, vec![level(2, 3)]),
            demon(3, vec![initial(3, 1)]),
            demon(4, vec![initial(4, 1)]),
            demon(5, vec![level(3, 2)]),
            demon(6, vec![initial(4, 1)]),
            demon(7, Vec::new()),
            demon(8, Vec::new()),
            demon(9, Vec::new()),
            demon(10, vec![initial(3, 1)]),
        ]),
        SkillCatalog::new(vec![
            skill(1, true),
            skill(2, true),
            skill(3, true),
            skill(4, false),
        ]),
        SpecialRecipeCatalog::new(vec![
            recipe(5, &[1, 2]),
            recipe(6, &[5, 3]),
            recipe(7, &[6, 4]),
            recipe(8, &[1, 2, 3, 4]),
            recipe(9, &[10, 1]),
            recipe(10, &[9, 2]),
        ]),
    )
}

fn demon(id: u32, natural_skills: Vec<NaturalSkill>) -> DemonMeta {
    DemonMeta {
        id: DemonId(id),
        race: Race::Fiend,
        base_level: 1,
        content: DemonContent::Base,
        compendium_price: (id * 37 % 13) * 100 + 1,
        natural_skills,
        innate_skill: SkillId(99),
    }
}

fn skill(id: u32, inheritable: bool) -> Skill {
    Skill {
        id: SkillId(id),
        category: SkillCategory::Physical,
        cost: 0,
        rank: 0,
        inheritable,
        flags: SkillFlags::default(),
    }
}

fn initial(skill: u32, order: u32) -> NaturalSkill {
    NaturalSkill {
        skill: SkillId(skill),
        acquisition: SkillAcquisition::Initial { order },
    }
}

fn level(skill: u32, level: u32) -> NaturalSkill {
    NaturalSkill {
        skill: SkillId(skill),
        acquisition: SkillAcquisition::Level { level },
    }
}

fn recipe(result: u32, materials: &[u32]) -> RecipeMeta {
    RecipeMeta {
        result: DemonId(result),
        materials: materials.iter().copied().map(DemonId).collect(),
        is_special: true,
    }
}
