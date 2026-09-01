use std::time::Instant;

use smt5_fusion_core::{
    data::game_data::GameData,
    dataset::{self, demon_ids, skill_ids},
    model::{
        demon::{DemonContent, DemonMeta, SkillAcquisition},
        player_context::PlayerContext,
        route::{FusionSubroute, Route},
        skill::{Skill, SkillCategory, SkillId},
    },
    reverse_search::{SearchError, SearchRequest, SearchSafetyLimit, SearchSolution, search},
    route_replay::replay,
};

#[derive(Debug, Clone, Copy)]
struct DlcConfiguration {
    konohana_sakuya: bool,
    dagda: bool,
}

const DLC_CONFIGURATIONS: [DlcConfiguration; 4] = [
    DlcConfiguration {
        konohana_sakuya: false,
        dagda: false,
    },
    DlcConfiguration {
        konohana_sakuya: true,
        dagda: false,
    },
    DlcConfiguration {
        konohana_sakuya: false,
        dagda: true,
    },
    DlcConfiguration {
        konohana_sakuya: true,
        dagda: true,
    },
];

#[test]
fn formal_depth_zero_matches_the_independent_oracle_for_every_target_and_skill() {
    let data = dataset::game_data();
    let context = prepared_context(
        &data,
        DlcConfiguration {
            konohana_sakuya: true,
            dagda: true,
        },
    );
    let supported_skills = supported_skills(&data);
    let mut request_count = 0usize;

    for demon in data.demons().iter() {
        assert_search_matches_oracle(
            &data,
            &context,
            &SearchRequest {
                target: demon.id,
                required_skills: Vec::new(),
                max_fusion_depth: 0,
            },
        );
        request_count += 1;

        for &skill in &supported_skills {
            assert_search_matches_oracle(
                &data,
                &context,
                &SearchRequest {
                    target: demon.id,
                    required_skills: vec![skill],
                    max_fusion_depth: 0,
                },
            );
            request_count += 1;
        }

        let natural_skills = supported_natural_skills(&data, demon);
        assert_search_matches_oracle(
            &data,
            &context,
            &SearchRequest {
                target: demon.id,
                required_skills: natural_skills,
                max_fusion_depth: 0,
            },
        );
        request_count += 1;
    }

    assert_eq!(request_count, 96_525);
}

#[test]
fn formal_depth_zero_covers_every_dlc_configuration_and_unsupported_skill() {
    let data = dataset::game_data();

    for configuration in DLC_CONFIGURATIONS {
        let context = prepared_context(&data, configuration);
        for demon in data.demons().iter() {
            for required_skills in [Vec::new(), supported_natural_skills(&data, demon)] {
                let request = SearchRequest {
                    target: demon.id,
                    required_skills,
                    max_fusion_depth: 0,
                };
                if is_available(demon, configuration) {
                    assert_search_matches_oracle(&data, &context, &request);
                } else {
                    assert_eq!(
                        search(&data, &context, &request),
                        Err(SearchError::UnavailableTarget(demon.id)),
                        "configuration={configuration:?}, request={request:?}"
                    );
                }
            }
        }
    }

    let context = prepared_context(
        &data,
        DlcConfiguration {
            konohana_sakuya: true,
            dagda: true,
        },
    );
    let target = demon_ids::PIXIE;
    let mut unsupported_count = 0usize;
    for skill in data.skills().iter().filter(|skill| !is_supported(skill)) {
        let request = SearchRequest {
            target,
            required_skills: vec![skill.id],
            max_fusion_depth: 0,
        };
        assert_eq!(
            search(&data, &context, &request),
            Err(SearchError::UnsupportedSkill(skill.id)),
            "request={request:?}"
        );
        unsupported_count += 1;
    }
    assert_eq!(unsupported_count, 414);
}

#[test]
fn formal_depth_one_empty_skill_routes_match_every_direct_recipe() {
    let data = dataset::game_data();
    let expected_recipe_counts = [30_138usize, 30_352, 30_352, 30_567];

    for (configuration, expected_recipe_count) in
        DLC_CONFIGURATIONS.into_iter().zip(expected_recipe_counts)
    {
        let context = prepared_context(&data, configuration);
        let mut actual_recipe_count = 0usize;

        for demon in data.demons().iter() {
            let request = SearchRequest {
                target: demon.id,
                required_skills: Vec::new(),
                max_fusion_depth: 1,
            };
            if is_available(demon, configuration) {
                let solutions = assert_search_matches_oracle(&data, &context, &request);
                actual_recipe_count += solutions
                    .iter()
                    .filter(|solution| solution.fusion_depth == 1)
                    .count();
            } else {
                assert_eq!(
                    search(&data, &context, &request),
                    Err(SearchError::UnavailableTarget(demon.id)),
                    "configuration={configuration:?}, request={request:?}"
                );
            }
        }

        assert_eq!(
            actual_recipe_count, expected_recipe_count,
            "configuration={configuration:?}"
        );
    }
}

#[test]
fn formal_depth_one_skill_corpus_matches_the_independent_oracle() {
    let data = dataset::game_data();
    let contexts = DLC_CONFIGURATIONS.map(|configuration| prepared_context(&data, configuration));
    let all_enabled = 3usize;
    let probe_skills = [
        skill_ids::AGI,
        skill_ids::MARAGIBARION,
        skill_ids::HAMABARION,
        skill_ids::LUSTER_CANDY,
        skill_ids::ENDURING_SOUL,
        skill_ids::SAFEGUARD,
        skill_ids::SAKUYA_SAKURA,
    ];
    let skill_targets = [
        demon_ids::HIGH_PIXIE,
        demon_ids::SHIVA,
        demon_ids::SATAN,
        demon_ids::ALICE,
        demon_ids::AMABIE,
        demon_ids::KONOHANA_SAKUYA,
        demon_ids::DAGDA,
    ];
    let mut corpus = Vec::new();

    for (configuration_index, _) in DLC_CONFIGURATIONS.iter().enumerate() {
        for demon in data.demons().iter() {
            for skill in probe_skills {
                corpus.push((
                    configuration_index,
                    SearchRequest {
                        target: demon.id,
                        required_skills: vec![skill],
                        max_fusion_depth: 1,
                    },
                ));
            }
        }
    }

    for (index, skill) in supported_skills(&data).into_iter().enumerate() {
        corpus.push((
            all_enabled,
            SearchRequest {
                target: skill_targets[index % skill_targets.len()],
                required_skills: vec![skill],
                max_fusion_depth: 1,
            },
        ));
    }

    for demon in data.demons().iter() {
        corpus.push((
            all_enabled,
            SearchRequest {
                target: demon.id,
                required_skills: supported_natural_skills(&data, demon),
                max_fusion_depth: 1,
            },
        ));
    }

    for (target, required_skills) in [
        (
            demon_ids::SHIVA,
            vec![skill_ids::DAMASCUS_CLAW, skill_ids::NIHIL_CLAW],
        ),
        (
            demon_ids::SATAN,
            vec![
                skill_ids::TRISAGION,
                skill_ids::POISON_MASTER,
                skill_ids::DARK_SWORD,
            ],
        ),
        (
            demon_ids::ALICE,
            vec![
                skill_ids::MAZANMA,
                skill_ids::SHOCKBOUND,
                skill_ids::MAMUDO,
                skill_ids::MARAGI,
            ],
        ),
        (
            demon_ids::HIGH_PIXIE,
            vec![skill_ids::AGI, skill_ids::BUFU, skill_ids::ZIO],
        ),
        (
            demon_ids::AMABIE,
            vec![
                skill_ids::AGI,
                skill_ids::BUFU,
                skill_ids::ZIO,
                skill_ids::ZAN,
                skill_ids::HAMA,
                skill_ids::MUDO,
                skill_ids::DIA,
                skill_ids::LUSTER_CANDY,
            ],
        ),
    ] {
        corpus.push((
            all_enabled,
            SearchRequest {
                target,
                required_skills,
                max_fusion_depth: 1,
            },
        ));
    }

    let mut route_count = 0usize;
    for (configuration_index, request) in &corpus {
        let configuration = DLC_CONFIGURATIONS[*configuration_index];
        let context = &contexts[*configuration_index];
        let target = data.demons().get(request.target).unwrap();
        if is_available(target, configuration) {
            route_count += assert_search_matches_oracle(&data, context, request).len();
        } else {
            assert_eq!(
                search(&data, context, request),
                Err(SearchError::UnavailableTarget(request.target)),
                "configuration={configuration:?}, request={request:?}"
            );
        }
    }

    assert_eq!(corpus.len(), 8_329);
    assert_eq!(route_count, 95_847);
}

#[test]
fn formal_depth_two_regression_cases_remain_replayable_and_stable() {
    let data = dataset::game_data();
    let context = prepared_context(
        &data,
        DlcConfiguration {
            konohana_sakuya: true,
            dagda: true,
        },
    );
    let cases = [
        (
            SearchRequest {
                target: demon_ids::HIGH_PIXIE,
                required_skills: Vec::new(),
                max_fusion_depth: 2,
            },
            [1, 1, 79],
            Fingerprint {
                first: 0x8cdb_5b8e_500b_58fc,
                second: 0xc4b0_9993_c6df_1bb7,
            },
        ),
        (
            SearchRequest {
                target: demon_ids::SHIVA,
                required_skills: vec![skill_ids::AGI],
                max_fusion_depth: 2,
            },
            [0, 0, 341],
            Fingerprint {
                first: 0x2a45_dc54_b669_42f7,
                second: 0xe1e9_a3cd_cbe7_c275,
            },
        ),
        (
            SearchRequest {
                target: demon_ids::SATAN,
                required_skills: vec![skill_ids::HAMABARION],
                max_fusion_depth: 2,
            },
            [0, 2, 18],
            Fingerprint {
                first: 0x2da5_9de0_cf83_bd9f,
                second: 0x9dbc_d71f_5d36_1532,
            },
        ),
        (
            SearchRequest {
                target: demon_ids::SATAN,
                required_skills: vec![
                    skill_ids::TRISAGION,
                    skill_ids::POISON_MASTER,
                    skill_ids::DARK_SWORD,
                ],
                max_fusion_depth: 2,
            },
            [0, 1, 7],
            Fingerprint {
                first: 0x94ca_c8af_8793_145c,
                second: 0x137b_ac5b_274e_73c9,
            },
        ),
        (
            SearchRequest {
                target: demon_ids::AMABIE,
                required_skills: vec![skill_ids::AGI],
                max_fusion_depth: 2,
            },
            [0, 0, 0],
            Fingerprint {
                first: 0xa8c7_f832_281a_39c5,
                second: 0x7367_d18c_f524_176b,
            },
        ),
    ];

    for (request, expected_depth_counts, expected_fingerprint) in cases {
        let first = search(&data, &context, &request).unwrap();
        assert_solution_depths(&first, expected_depth_counts, &request);
        assert_replayable(&data, &context, &request, &first);
        assert_eq!(
            solution_fingerprint(&first),
            expected_fingerprint,
            "request={request:?}"
        );
        assert_eq!(
            first,
            search(&data, &context, &request).unwrap(),
            "request={request:?}"
        );
    }
}

#[test]
fn formal_pressure_cases_report_the_current_safety_boundary() {
    let data = dataset::game_data();
    let context = prepared_context(
        &data,
        DlcConfiguration {
            konohana_sakuya: true,
            dagda: true,
        },
    );
    let cases = [
        SearchRequest {
            target: demon_ids::ALICE,
            required_skills: Vec::new(),
            max_fusion_depth: 2,
        },
        SearchRequest {
            target: demon_ids::PIXIE,
            required_skills: Vec::new(),
            max_fusion_depth: 3,
        },
        SearchRequest {
            target: demon_ids::SATAN,
            required_skills: vec![
                skill_ids::TRISAGION,
                skill_ids::POISON_MASTER,
                skill_ids::DARK_SWORD,
            ],
            max_fusion_depth: 3,
        },
    ];

    for request in cases {
        assert_eq!(
            search(&data, &context, &request),
            Err(SearchError::SafetyLimitExceeded(
                SearchSafetyLimit::RouteCombinations { maximum: 250_000 }
            )),
            "request={request:?}"
        );
    }
}

#[test]
#[ignore = "full four-DLC target-by-skill matrix; run in release mode"]
fn exhaustive_formal_depth_one_single_skill_matrix_matches_the_oracle() {
    let started = Instant::now();
    let data = dataset::game_data();
    let skills = supported_skills(&data);
    let mut request_count = 0usize;
    let mut route_count = 0usize;

    for configuration in DLC_CONFIGURATIONS {
        let context = prepared_context(&data, configuration);
        for demon in data
            .demons()
            .iter()
            .filter(|demon| is_available(demon, configuration))
        {
            for &skill in &skills {
                let request = SearchRequest {
                    target: demon.id,
                    required_skills: vec![skill],
                    max_fusion_depth: 1,
                };
                route_count += assert_search_matches_oracle(&data, &context, &request).len();
                request_count += 1;
            }
        }
    }

    eprintln!(
        "validated {request_count} formal depth-one requests and {route_count} routes in {:?}",
        started.elapsed()
    );
    assert_eq!(request_count, 382_504);
    assert_eq!(route_count, 1_995_666);
}

#[test]
#[ignore = "full depth-two target sweep; run in release mode"]
fn exhaustive_formal_depth_two_empty_skill_sweep_matches_the_baseline() {
    let started = Instant::now();
    let data = dataset::game_data();
    let context = prepared_context(
        &data,
        DlcConfiguration {
            konohana_sakuya: true,
            dagda: true,
        },
    );
    let mut successful_targets = 0usize;
    let mut limited_targets = 0usize;
    let mut route_count = 0usize;
    let mut fingerprinter = Fingerprinter::new();

    for demon in data.demons().iter() {
        let request = SearchRequest {
            target: demon.id,
            required_skills: Vec::new(),
            max_fusion_depth: 2,
        };
        fingerprinter.write_u32(demon.id.0);
        match search(&data, &context, &request) {
            Ok(solutions) => {
                fingerprinter.write_byte(0);
                fingerprinter.write_usize(solutions.len());
                for solution in &solutions {
                    fingerprinter.write_solution(solution);
                }
                assert_replayable(&data, &context, &request, &solutions);
                successful_targets += 1;
                route_count += solutions.len();
            }
            Err(SearchError::SafetyLimitExceeded(SearchSafetyLimit::RouteCombinations {
                maximum: 250_000,
            })) => {
                fingerprinter.write_byte(1);
                limited_targets += 1;
            }
            Err(error) => panic!("unexpected error for {request:?}: {error:?}"),
        }
    }

    eprintln!(
        "depth-two sweep: successful_targets={successful_targets}, limited_targets={limited_targets}, routes={route_count}, fingerprint={:?}, elapsed={:?}",
        fingerprinter.fingerprint,
        started.elapsed()
    );
    assert_eq!(successful_targets, 97);
    assert_eq!(limited_targets, 178);
    assert_eq!(route_count, 6_205_576);
    assert_eq!(
        fingerprinter.fingerprint,
        Fingerprint {
            first: 0x496f_652e_1a3d_c86d,
            second: 0x69bb_33cc_cf49_4617,
        }
    );
}

fn prepared_context(data: &GameData, configuration: DlcConfiguration) -> PlayerContext {
    let mut context = PlayerContext::default();
    context.set_konohana_sakuya_dlc(configuration.konohana_sakuya);
    context.set_dagda_dlc(configuration.dagda);
    context.prepare_direct_recipes(data);
    context
}

fn is_available(demon: &DemonMeta, configuration: DlcConfiguration) -> bool {
    match demon.content {
        DemonContent::Base => true,
        DemonContent::KonohanaSakuyaDlc => configuration.konohana_sakuya,
        DemonContent::DagdaDlc => configuration.dagda,
    }
}

fn is_supported(skill: &Skill) -> bool {
    skill.category != SkillCategory::Innate && !skill.flags.magatsuhi && !skill.flags.item_only
}

fn supported_skills(data: &GameData) -> Vec<SkillId> {
    data.skills()
        .iter()
        .filter(|skill| is_supported(skill))
        .map(|skill| skill.id)
        .collect()
}

fn supported_natural_skills(data: &GameData, demon: &DemonMeta) -> Vec<SkillId> {
    let mut skills = demon
        .natural_skills
        .iter()
        .map(|natural| natural.skill)
        .filter(|skill| data.skills().get(*skill).is_some_and(is_supported))
        .collect::<Vec<_>>();
    skills.sort_unstable();
    skills.dedup();
    skills
}

fn assert_search_matches_oracle(
    data: &GameData,
    context: &PlayerContext,
    request: &SearchRequest,
) -> Vec<SearchSolution> {
    assert!(request.max_fusion_depth <= 1);
    let expected = oracle_routes(data, context, request);
    let actual = search(data, context, request).unwrap_or_else(|error| {
        panic!("search failed for {request:?}: {error:?}");
    });

    assert_same_routes(&actual, &expected, request);
    assert_replayable(data, context, request, &actual);
    actual
}

fn oracle_routes(data: &GameData, context: &PlayerContext, request: &SearchRequest) -> Vec<Route> {
    let target = data.demons().get(request.target).unwrap();
    let required_skills = normalized_skills(&request.required_skills);
    let mut routes = depth_zero_route(target, &required_skills)
        .into_iter()
        .collect::<Vec<_>>();

    if request.max_fusion_depth == 1 {
        routes.extend(depth_one_routes(data, context, target, &required_skills));
    }
    routes
}

fn depth_zero_route(demon: &DemonMeta, required_skills: &[SkillId]) -> Option<Route> {
    let mut target_level = demon.base_level;
    for required_skill in required_skills {
        let acquisition = demon
            .natural_skills
            .iter()
            .find(|natural| natural.skill == *required_skill)?
            .acquisition;
        if let SkillAcquisition::Level { level } = acquisition {
            target_level = target_level.max(level);
        }
    }

    Some(add_upgrades(
        demon.base_level,
        target_level,
        Route::Direct { demon: demon.id },
    ))
}

fn depth_one_routes(
    data: &GameData,
    context: &PlayerContext,
    target: &DemonMeta,
    required_skills: &[SkillId],
) -> Vec<Route> {
    let mut routes = Vec::new();

    for (target_level, local_skills) in target_levels(target, required_skills) {
        let inherited_skills = required_skills
            .iter()
            .copied()
            .filter(|skill| !local_skills.contains(skill))
            .collect::<Vec<_>>();
        if inherited_skills
            .iter()
            .any(|skill| !data.skills().get(*skill).unwrap().inheritable)
        {
            continue;
        }

        for recipe in context.get_direct_recipes(target.id).unwrap_or_default() {
            for assignment in skill_assignments(&inherited_skills, recipe.materials.len()) {
                let materials = recipe
                    .materials
                    .iter()
                    .copied()
                    .zip(assignment)
                    .map(|(material, required)| {
                        let material_meta = data.demons().get(material).unwrap();
                        depth_zero_route(material_meta, &required).map(|route| FusionSubroute {
                            required_skills: required,
                            route,
                        })
                    })
                    .collect::<Option<Vec<_>>>();
                let Some(materials) = materials else {
                    continue;
                };

                routes.push(add_upgrades(
                    target.base_level,
                    target_level,
                    Route::Fusion {
                        recipe: recipe.clone(),
                        materials,
                    },
                ));
            }
        }
    }

    routes
}

fn target_levels(demon: &DemonMeta, required_skills: &[SkillId]) -> Vec<(u32, Vec<SkillId>)> {
    let maximum_level = demon
        .natural_skills
        .iter()
        .filter(|natural| required_skills.contains(&natural.skill))
        .filter_map(|natural| match natural.acquisition {
            SkillAcquisition::Initial { .. } => None,
            SkillAcquisition::Level { level } => Some(level),
        })
        .max()
        .unwrap_or(demon.base_level);
    let mut levels = Vec::new();

    for level in demon.base_level..=maximum_level {
        let local_skills = required_skills
            .iter()
            .copied()
            .filter(|skill| acquired_by_level(demon, *skill, level))
            .collect::<Vec<_>>();
        if levels
            .last()
            .is_none_or(|(_, previous)| *previous != local_skills)
        {
            levels.push((level, local_skills));
        }
    }
    levels
}

fn acquired_by_level(demon: &DemonMeta, skill: SkillId, level: u32) -> bool {
    demon
        .natural_skills
        .iter()
        .find(|natural| natural.skill == skill)
        .is_some_and(|natural| match natural.acquisition {
            SkillAcquisition::Initial { .. } => true,
            SkillAcquisition::Level {
                level: learned_level,
            } => learned_level <= level,
        })
}

fn skill_assignments(skills: &[SkillId], material_count: usize) -> Vec<Vec<Vec<SkillId>>> {
    fn visit(
        skills: &[SkillId],
        skill_index: usize,
        current: &mut [Vec<SkillId>],
        assignments: &mut Vec<Vec<Vec<SkillId>>>,
    ) {
        if skill_index == skills.len() {
            assignments.push(current.to_vec());
            return;
        }

        for material_index in 0..current.len() {
            current[material_index].push(skills[skill_index]);
            visit(skills, skill_index + 1, current, assignments);
            current[material_index].pop();
        }
    }

    if material_count == 0 {
        return Vec::new();
    }
    let mut assignments = Vec::new();
    visit(
        skills,
        0,
        &mut vec![Vec::new(); material_count],
        &mut assignments,
    );
    assignments
}

fn normalized_skills(skills: &[SkillId]) -> Vec<SkillId> {
    let mut normalized = skills.to_vec();
    normalized.sort_unstable();
    normalized.dedup();
    normalized
}

fn add_upgrades(base_level: u32, target_level: u32, mut route: Route) -> Route {
    for level in base_level + 1..=target_level {
        route = Route::Upgrade {
            level,
            previous: Box::new(route),
        };
    }
    route
}

fn assert_same_routes(actual: &[SearchSolution], expected: &[Route], request: &SearchRequest) {
    let difference = actual
        .iter()
        .map(|solution| &solution.route)
        .zip(expected)
        .position(|(actual, expected)| actual != expected);
    if actual.len() != expected.len() || difference.is_some() {
        let index = difference.unwrap_or(actual.len().min(expected.len()));
        panic!(
            "route mismatch for {request:?}: actual_count={}, expected_count={}, first_difference={}, actual={:?}, expected={:?}",
            actual.len(),
            expected.len(),
            index,
            actual.get(index).map(|solution| &solution.route),
            expected.get(index)
        );
    }

    for (index, solution) in actual.iter().enumerate() {
        assert!(
            actual[..index]
                .iter()
                .all(|previous| previous.route != solution.route),
            "duplicate route for {request:?}: {:?}",
            solution.route
        );
    }
}

fn assert_replayable(
    data: &GameData,
    context: &PlayerContext,
    request: &SearchRequest,
    solutions: &[SearchSolution],
) {
    for solution in solutions {
        assert_eq!(
            solution.fusion_depth,
            route_depth(&solution.route),
            "request={request:?}"
        );
        assert_eq!(
            replay(data, context, request, &solution.route),
            Ok(solution.demon.clone()),
            "request={request:?}"
        );
    }
}

fn assert_solution_depths<const N: usize>(
    solutions: &[SearchSolution],
    expected: [usize; N],
    request: &SearchRequest,
) {
    let mut actual = [0usize; N];
    for solution in solutions {
        actual[solution.fusion_depth as usize] += 1;
    }
    assert_eq!(actual, expected, "request={request:?}");
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Fingerprint {
    first: u64,
    second: u64,
}

struct Fingerprinter {
    fingerprint: Fingerprint,
}

impl Fingerprinter {
    fn new() -> Self {
        Self {
            fingerprint: Fingerprint {
                first: 0xcbf2_9ce4_8422_2325,
                second: 0x8422_2325_cbf2_9ce4,
            },
        }
    }

    fn write_byte(&mut self, byte: u8) {
        self.fingerprint.first ^= u64::from(byte);
        self.fingerprint.first = self.fingerprint.first.wrapping_mul(0x0000_0100_0000_01b3);
        self.fingerprint.second = self
            .fingerprint
            .second
            .wrapping_mul(0x9e37_79b1_85eb_ca87)
            .rotate_left(7)
            ^ u64::from(byte);
    }

    fn write_u32(&mut self, value: u32) {
        for byte in value.to_le_bytes() {
            self.write_byte(byte);
        }
    }

    fn write_usize(&mut self, value: usize) {
        for byte in (value as u64).to_le_bytes() {
            self.write_byte(byte);
        }
    }

    fn write_solution(&mut self, solution: &SearchSolution) {
        self.write_byte(0xff);
        self.write_u32(solution.fusion_depth);
        self.write_u32(solution.demon.meta.0);
        self.write_u32(solution.demon.level);
        self.write_usize(solution.demon.skills.len());
        for skill in &solution.demon.skills {
            self.write_u32(skill.0);
        }
        self.write_route(&solution.route);
    }

    fn write_route(&mut self, route: &Route) {
        match route {
            Route::Direct { demon } => {
                self.write_byte(0);
                self.write_u32(demon.0);
            }
            Route::Upgrade { level, previous } => {
                self.write_byte(1);
                self.write_u32(*level);
                self.write_route(previous);
            }
            Route::Fusion { recipe, materials } => {
                self.write_byte(2);
                self.write_u32(recipe.result.0);
                self.write_byte(u8::from(recipe.is_special));
                self.write_usize(recipe.materials.len());
                for material in &recipe.materials {
                    self.write_u32(material.0);
                }
                self.write_usize(materials.len());
                for material in materials {
                    self.write_usize(material.required_skills.len());
                    for skill in &material.required_skills {
                        self.write_u32(skill.0);
                    }
                    self.write_route(&material.route);
                }
            }
        }
    }
}

fn solution_fingerprint(solutions: &[SearchSolution]) -> Fingerprint {
    let mut fingerprinter = Fingerprinter::new();
    fingerprinter.write_usize(solutions.len());
    for solution in solutions {
        fingerprinter.write_solution(solution);
    }
    fingerprinter.fingerprint
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
