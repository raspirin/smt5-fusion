use std::{collections::BTreeSet, sync::Arc};

use leptos::prelude::*;

use crate::{
    i18n::{I18n, Locale},
    protocol::{
        AcquisitionDto, DemonId, DlcSettingsDto, NodeOptionsDto, OptionAcquisitionDto,
        RouteTreeNodeDto, SearchInputDto, SearchResultDto, SkillId, VisibleOptionDto,
        WorkerFailureCode, WorkerFailureDto, WorkerResponse,
    },
};

use super::{
    selectors::{
        all_collapsible_paths, filtered_demons, filtered_skills, grouped_digits, option_matches,
    },
    state::{AppError, AppState, Controller, PersistedForm},
};
use crate::service::WorkerService;

#[cfg(not(target_arch = "wasm32"))]
mod interactions;

#[test]
fn big_counts_are_grouped_without_losing_precision() {
    assert_eq!(grouped_digits("0"), "0");
    assert_eq!(grouped_digits("999"), "999");
    assert_eq!(grouped_digits("1000"), "1,000");
    assert_eq!(
        grouped_digits("124658498600493251948484585761611474422341723641781926782383854"),
        "124,658,498,600,493,251,948,484,585,761,611,474,422,341,723,641,781,926,782,383,854"
    );
}

#[test]
fn source_search_matches_only_demons_present_in_an_option() {
    let direct = VisibleOptionDto {
        option_id: 0,
        selected: false,
        acquisition: OptionAcquisitionDto::Direct {
            summon_level: 1,
            target_level: 1,
        },
    };
    let fusion = VisibleOptionDto {
        option_id: 1,
        selected: false,
        acquisition: OptionAcquisitionDto::Fusion {
            is_special: false,
            route_depth: 1,
            fusion_level: 1,
            target_level: 1,
            materials: vec![crate::protocol::OptionMaterialDto {
                demon: DemonId(32),
                required_skills: Vec::new(),
            }],
        },
    };

    assert!(option_matches(&direct, "", Locale::ZhCn));
    assert!(!option_matches(&direct, "巴隆", Locale::ZhCn));
    assert!(option_matches(&fusion, "巴隆", Locale::ZhCn));
    assert!(option_matches(&fusion, "Barong", Locale::ZhCn));
    assert!(option_matches(&fusion, "バロン", Locale::ZhCn));
    assert!(!option_matches(&fusion, "湿婆", Locale::ZhCn));
    assert!(option_matches(&fusion, "barong", Locale::EnUs));
}

#[test]
fn main_pickers_search_names_across_locales() {
    Owner::new().with(|| {
        let state = AppState::new(PersistedForm::default(), I18n::new(Locale::ZhCn));
        state.handle_response(WorkerService::new().ready());

        for query in ["Shiva", "シヴァ", "湿婆", "濕婆", "ｼｳﾞｧ"] {
            state.target_query.set(query.to_owned());
            assert_eq!(
                filtered_demons(state).first().map(|demon| demon.id),
                Some(DemonId(193))
            );
        }

        state.target.set(Some(DemonId(193)));
        state.required_skills.set(Vec::new());
        for query in ["Riberama", "リベラマ", "利悖拉玛", "利悖拉瑪", "ﾘﾍﾞﾗﾏ"]
        {
            state.skill_query.set(query.to_owned());
            assert_eq!(
                filtered_skills(state).first().map(|skill| skill.id),
                Some(SkillId(745))
            );
        }
    });
}

#[test]
fn collapse_all_collects_every_node_with_materials() {
    fn node(path: &[u8], children: Vec<RouteTreeNodeDto>) -> RouteTreeNodeDto {
        RouteTreeNodeDto {
            path: path.to_vec(),
            demon: DemonId(0),
            base_level: 1,
            final_level: 1,
            required_skills: Vec::new(),
            upgrade_skills: Vec::new(),
            acquisition: AcquisitionDto::Direct {
                summon_level: 1,
                target_level: 1,
            },
            can_change_recipe: false,
            children,
        }
    }

    let tree = node(
        &[],
        vec![
            node(&[0], vec![node(&[0, 0], Vec::new())]),
            node(&[1], Vec::new()),
        ],
    );

    assert_eq!(
        all_collapsible_paths(&tree),
        BTreeSet::from([Vec::new(), vec![0]])
    );
}

#[test]
fn activating_the_current_recipe_closes_its_picker() {
    Owner::new().with(|| {
        let state = AppState::new(PersistedForm::default(), I18n::new(Locale::ZhCn));
        state.begin_options_request(7, vec![0]);
        state.reveal_options_indicator(7);

        Controller::new(state).select_option(0, true);

        assert!(state.panel_path.get_untracked().is_none());
        assert!(!state.options_indicator_visible.get_untracked());
        assert!(state.active_options.get_untracked().is_none());
    });
}

#[test]
fn selected_depth_is_copied_into_the_search_input() {
    Owner::new().with(|| {
        let state = AppState::new(PersistedForm::default(), I18n::new(Locale::ZhCn));
        state.target.set(Some(DemonId(193)));
        let controller = Controller::new(state);

        for depth in 0..=4 {
            controller.set_depth(depth);
            assert_eq!(
                state.current_input().map(|input| input.max_fusion_depth),
                Some(depth)
            );
        }
    });
}

#[test]
fn clearing_the_form_also_clears_the_route() {
    Owner::new().with(|| {
        let state = AppState::new(PersistedForm::default(), I18n::new(Locale::ZhCn));
        state.target.set(Some(DemonId(193)));
        state.konohana_sakuya_dlc.set(true);
        state.dagda_dlc.set(true);
        state.result.set(Some(Arc::new(SearchResultDto {
            session_id: 1,
            selection_revision: 0,
            input: SearchInputDto {
                target: DemonId(193),
                required_skills: Vec::new(),
                max_fusion_depth: 2,
                dlc: DlcSettingsDto::default(),
            },
            route_count: "1".to_owned(),
            actual_fusion_depth: 0,
            tree: None,
        })));
        state.active_search.set(Some(7));
        state.searching.set(true);
        state.search_indicator_visible.set(true);
        state.skill_picker_open.set(true);
        state.skill_query.set("test".to_owned());
        state
            .skill_category
            .set(Some(crate::protocol::SkillCategory::Physical));

        Controller::new(state).clear_form();

        assert!(state.target.get_untracked().is_none());
        assert!(state.result.get_untracked().is_none());
        assert!(state.active_search.get_untracked().is_none());
        assert!(!state.searching.get_untracked());
        assert!(!state.search_indicator_visible.get_untracked());
        assert!(state.konohana_sakuya_dlc.get_untracked());
        assert!(state.dagda_dlc.get_untracked());
        assert!(!untrack(|| state.result_is_stale()));
        assert!(!state.skill_picker_open.get_untracked());
        assert!(state.skill_query.get_untracked().is_empty());
        assert!(state.skill_category.get_untracked().is_none());
    });
}

#[test]
fn delayed_search_indicator_requires_the_active_request() {
    Owner::new().with(|| {
        let state = AppState::new(PersistedForm::default(), I18n::new(Locale::ZhCn));
        state.active_search.set(Some(7));
        state.searching.set(true);

        state.reveal_search_indicator(6);
        assert!(!state.search_indicator_visible.get_untracked());

        state.reveal_search_indicator(7);
        assert!(state.search_indicator_visible.get_untracked());

        state.search_indicator_visible.set(false);
        state.searching.set(false);
        state.reveal_search_indicator(7);
        assert!(!state.search_indicator_visible.get_untracked());
    });
}

#[test]
fn delayed_options_indicator_requires_an_open_active_request() {
    Owner::new().with(|| {
        let state = AppState::new(PersistedForm::default(), I18n::new(Locale::ZhCn));
        state.begin_options_request(7, vec![0]);
        assert_eq!(state.panel_path.get_untracked(), Some(vec![0]));
        assert!(!state.options_indicator_visible.get_untracked());

        state.reveal_options_indicator(6);
        assert!(!state.options_indicator_visible.get_untracked());
        state.reveal_options_indicator(7);
        assert!(state.options_indicator_visible.get_untracked());

        state.close_options();
        state.reveal_options_indicator(7);
        assert!(!state.options_indicator_visible.get_untracked());
        assert!(state.active_options.get_untracked().is_none());
    });
}

#[test]
fn switching_nodes_restarts_the_options_indicator_delay() {
    Owner::new().with(|| {
        let state = AppState::new(PersistedForm::default(), I18n::new(Locale::ZhCn));
        state.begin_options_request(7, vec![0]);
        state.reveal_options_indicator(7);
        assert!(state.options_indicator_visible.get_untracked());

        state.begin_options_request(8, vec![1]);
        state.reveal_options_indicator(7);
        state.handle_response(WorkerResponse::NodeOptions {
            request_id: 7,
            options: NodeOptionsDto {
                session_id: 1,
                selection_revision: 0,
                path: vec![0],
                demon: DemonId(193),
                options: Vec::new(),
            },
        });
        assert_eq!(state.panel_path.get_untracked(), Some(vec![1]));
        assert_eq!(state.active_options.get_untracked(), Some(8));
        assert!(state.options.get_untracked().is_none());
        assert!(!state.options_indicator_visible.get_untracked());

        state.reveal_options_indicator(8);
        assert!(state.options_indicator_visible.get_untracked());
    });
}

#[test]
fn completed_options_requests_hide_the_indicator_and_ignore_late_timers() {
    for delay_elapsed in [false, true] {
        Owner::new().with(|| {
            let state = AppState::new(PersistedForm::default(), I18n::new(Locale::ZhCn));
            let result = Arc::new(SearchResultDto {
                session_id: 1,
                selection_revision: 0,
                input: SearchInputDto {
                    target: DemonId(193),
                    required_skills: Vec::new(),
                    max_fusion_depth: 2,
                    dlc: DlcSettingsDto::default(),
                },
                route_count: "1".to_owned(),
                actual_fusion_depth: 0,
                tree: None,
            });
            state.result.set(Some(Arc::clone(&result)));
            state.session_available.set(true);
            state.begin_options_request(7, Vec::new());
            if delay_elapsed {
                state.reveal_options_indicator(7);
            }
            assert_eq!(
                state.options_indicator_visible.get_untracked(),
                delay_elapsed
            );

            let options = NodeOptionsDto {
                session_id: 1,
                selection_revision: 0,
                path: Vec::new(),
                demon: DemonId(193),
                options: vec![VisibleOptionDto {
                    option_id: 0,
                    selected: true,
                    acquisition: OptionAcquisitionDto::Direct {
                        summon_level: 82,
                        target_level: 82,
                    },
                }],
            };
            state.handle_response(WorkerResponse::NodeOptions {
                request_id: 7,
                options: options.clone(),
            });
            state.reveal_options_indicator(7);

            assert!(!state.options_indicator_visible.get_untracked());
            assert!(state.active_options.get_untracked().is_none());
            assert_eq!(state.panel_path.get_untracked(), Some(Vec::new()));
            assert_eq!(state.options.get_untracked().as_deref(), Some(&options));
            assert!(Arc::ptr_eq(&state.result.get_untracked().unwrap(), &result));
        });
    }
}

#[test]
fn cancelled_options_requests_cannot_reveal_the_indicator() {
    let cancellations: [fn(&Controller); 4] = [
        Controller::clear_form,
        Controller::start_worker,
        |controller| controller.set_locale(Locale::EnUs),
        |controller| {
            controller.state.handle_response(WorkerResponse::Failure {
                request_id: Some(7),
                failure: WorkerFailureDto {
                    code: WorkerFailureCode::InvalidSession,
                    related_id: None,
                    selected: None,
                    maximum: None,
                },
            });
        },
    ];
    for cancel in cancellations {
        for delay_elapsed in [false, true] {
            Owner::new().with(|| {
                let state = AppState::new(PersistedForm::default(), I18n::new(Locale::ZhCn));
                state.begin_options_request(7, vec![0]);
                if delay_elapsed {
                    state.reveal_options_indicator(7);
                }

                cancel(&Controller::new(state));
                state.reveal_options_indicator(7);

                assert!(!state.options_indicator_visible.get_untracked());
                assert!(state.panel_path.get_untracked().is_none());
                assert!(state.active_options.get_untracked().is_none());
            });
        }
    }
}

#[test]
fn changing_locale_preserves_the_current_search_state() {
    Owner::new().with(|| {
        let state = AppState::new(PersistedForm::default(), I18n::new(Locale::ZhCn));
        state.target.set(Some(DemonId(193)));
        state
            .required_skills
            .set(vec![crate::protocol::SkillId(745)]);
        state.max_depth.set(4);
        state.result.set(Some(Arc::new(SearchResultDto {
            session_id: 17,
            selection_revision: 3,
            input: SearchInputDto {
                target: DemonId(193),
                required_skills: vec![crate::protocol::SkillId(745)],
                max_fusion_depth: 4,
                dlc: DlcSettingsDto::default(),
            },
            route_count: "67660618831471573302955414412367".to_owned(),
            actual_fusion_depth: 4,
            tree: None,
        })));
        state.target_picker_open.set(true);
        state.skill_picker_open.set(true);
        state.option_query.set("湿婆".to_owned());
        state.active_search.set(Some(23));
        state.searching.set(true);

        Controller::new(state).set_locale(Locale::EnUs);

        assert_eq!(state.i18n.locale_untracked(), Locale::EnUs);
        assert_eq!(state.target.get_untracked(), Some(DemonId(193)));
        assert_eq!(
            state.required_skills.get_untracked(),
            vec![crate::protocol::SkillId(745)]
        );
        assert_eq!(state.max_depth.get_untracked(), 4);
        let result = state.result.get_untracked().expect("result must remain");
        assert_eq!(result.session_id, 17);
        assert_eq!(result.selection_revision, 3);
        assert_eq!(result.route_count, "67660618831471573302955414412367");
        assert_eq!(state.target_query.get_untracked(), "Shiva");
        assert_eq!(state.active_search.get_untracked(), Some(23));
        assert!(state.searching.get_untracked());
        assert!(!untrack(|| state.result_is_stale()));
        assert!(!state.target_picker_open.get_untracked());
        assert!(!state.skill_picker_open.get_untracked());
        assert!(state.option_query.get_untracked().is_empty());
    });
}

#[test]
fn structured_errors_follow_the_current_locale() {
    Owner::new().with(|| {
        let i18n = I18n::new(Locale::ZhCn);
        let error = AppError::WorkerFailure(WorkerFailureDto {
            code: WorkerFailureCode::TooManySkills,
            related_id: None,
            selected: Some(9),
            maximum: Some(8),
        });
        let localized = Memo::new(move |_| error.localized(i18n));

        assert_eq!(
            localized.get_untracked(),
            "选择了 9 个技能，最多只能保留 8 个。"
        );
        i18n.set_locale(Locale::EnUs);
        assert_eq!(
            localized.get_untracked(),
            "You selected 9 skills; at most 8 can be kept."
        );
    });
}

#[test]
fn clearing_the_form_does_not_reset_the_locale() {
    Owner::new().with(|| {
        let state = AppState::new(PersistedForm::default(), I18n::new(Locale::JaJp));
        Controller::new(state).clear_form();
        assert_eq!(state.i18n.locale_untracked(), Locale::JaJp);
    });
}

#[test]
fn default_form_matches_product_defaults() {
    let form = PersistedForm::default();
    assert_eq!(form.max_depth, 2);
    assert!(!form.konohana_sakuya_dlc);
    assert!(!form.dagda_dlc);
    assert!(form.required_skills.is_empty());
}
