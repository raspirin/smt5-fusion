use super::*;

use smt5_fusion_core::dataset::{demon_ids, skill_ids};

use super::super::{
    selectors::{filtered_options, source_active_descendant},
    state::WorkerStatus,
    theme::{ThemePreference, ThemeState},
    worker_client::TestWorker,
};
use crate::protocol::{DemonContent, SelectionSnapshotDto, WorkerRequest};

fn setup() -> (Controller, TestWorker, WorkerService) {
    let state = AppState::new(PersistedForm::default(), I18n::new(Locale::ZhCn));
    let controller = Controller::new(state);
    controller.start_worker();
    let worker = controller.test_worker();
    let service = WorkerService::new();
    worker.respond(service.ready());
    controller.select_target(Some(demon_ids::SHIVA));
    state.required_skills.set(Vec::new());
    controller.set_depth(1);
    (controller, worker, service)
}

fn request(worker: &TestWorker) -> WorkerRequest {
    let mut requests = worker.take_requests();
    assert_eq!(requests.len(), 1);
    requests.pop().unwrap()
}

fn calculate(controller: &Controller, worker: &TestWorker, service: &mut WorkerService) {
    controller.search();
    worker.respond(service.handle(request(worker)));
    assert!(controller.state.result.get_untracked().is_some());
    assert!(controller.state.session_available.get_untracked());
}

#[test]
fn theme_changes_preserve_running_searches_results_and_expansion() {
    Owner::new().with(|| {
        let (controller, worker, mut service) = setup();
        let state = controller.state;
        let theme = ThemeState::new(ThemePreference::Auto, false);
        calculate(&controller, &worker, &mut service);
        let previous = state.result.get_untracked().unwrap();
        let collapsed = BTreeSet::from([vec![0], vec![1, 0]]);
        state.collapsed.set(collapsed.clone());
        let input = state.current_input();
        controller.search();
        let pending = request(&worker);
        let active_search = state.active_search.get_untracked();
        let generation = state.worker_generation.get_untracked();

        for preference in ThemePreference::ALL {
            theme.set_preference(preference);
            for dark in [true, false] {
                theme.set_system_dark(dark);
                assert!(Arc::ptr_eq(
                    &state.result.get_untracked().unwrap(),
                    &previous
                ));
                assert_eq!(state.collapsed.get_untracked(), collapsed);
                assert_eq!(state.current_input(), input);
                assert_eq!(state.active_search.get_untracked(), active_search);
                assert_eq!(state.worker_generation.get_untracked(), generation);
                assert!(state.searching.get_untracked());
                assert!(!worker.terminated());
                assert!(worker.take_requests().is_empty());
            }
        }
        worker.respond(service.handle(pending));
        assert!(!state.searching.get_untracked());
        assert!(untrack(|| state.can_edit_route()));
    });
}

#[test]
fn language_changes_and_clearing_the_form_preserve_the_theme() {
    Owner::new().with(|| {
        let (controller, worker, _) = setup();
        let theme = ThemeState::new(ThemePreference::Auto, true);
        theme.set_preference(ThemePreference::Light);
        for locale in Locale::ALL {
            controller.set_locale(locale);
            controller.clear_form();
            assert_eq!(untrack(|| theme.preference()), ThemePreference::Light);
            assert_eq!(theme.resolved(), "light");
            assert_eq!(controller.state.i18n.locale_untracked(), locale);
            assert!(!worker.terminated());
            assert!(worker.take_requests().is_empty());
        }
    });
}

#[test]
fn failed_search_keeps_the_old_result_read_only_until_a_new_search_succeeds() {
    Owner::new().with(|| {
        let (controller, worker, mut service) = setup();
        let state = controller.state;
        calculate(&controller, &worker, &mut service);
        let previous = state.result.get_untracked().unwrap();
        assert!(untrack(|| state.can_edit_route()));

        controller.search();
        let WorkerRequest::Search {
            request_id,
            mut input,
        } = request(&worker)
        else {
            panic!()
        };
        assert!(Arc::ptr_eq(
            &state.result.get_untracked().unwrap(),
            &previous
        ));
        assert!(!untrack(|| state.can_edit_route()));
        input.target = demon_ids::KONOHANA_SAKUYA;
        worker.respond(service.handle(WorkerRequest::Search { request_id, input }));
        assert!(Arc::ptr_eq(
            &state.result.get_untracked().unwrap(),
            &previous
        ));
        assert_eq!(state.worker_status.get_untracked(), WorkerStatus::Ready);
        assert!(!state.session_available.get_untracked());
        assert!(!state.searching.get_untracked());
        assert!(!state.search_indicator_visible.get_untracked());
        assert!(untrack(|| state.can_search()));

        controller.open_options(Vec::new());
        controller.reset_default();
        assert!(worker.take_requests().is_empty());
        calculate(&controller, &worker, &mut service);
        assert!(untrack(|| state.can_edit_route()));
        assert_ne!(
            state.result.get_untracked().unwrap().session_id,
            previous.session_id
        );
    });
}

#[test]
fn search_disables_only_conflicting_operations_and_keeps_the_input_snapshot() {
    Owner::new().with(|| {
        let (controller, worker, mut service) = setup();
        let state = controller.state;
        calculate(&controller, &worker, &mut service);
        controller.search();
        let pending = request(&worker);
        let WorkerRequest::Search { request_id, .. } = &pending else {
            panic!()
        };
        state.reveal_search_indicator(*request_id);
        assert!(!untrack(|| state.can_search()));
        assert!(!untrack(|| state.can_edit_route()));
        assert!(untrack(|| state.can_edit_skills()));
        controller.search();
        controller.reset_default();
        controller.open_options(Vec::new());
        assert!(worker.take_requests().is_empty());

        controller.open_skill_picker(0);
        controller.select_skill(skill_ids::RIBERAMA);
        controller.set_depth(3);
        controller.set_locale(Locale::JaJp);
        assert!(!worker.terminated());
        assert!(state.search_indicator_visible.get_untracked());
        worker.respond(service.handle(pending));
        assert_eq!(
            state.required_skills.get_untracked(),
            vec![skill_ids::RIBERAMA]
        );
        let result = state.result.get_untracked().unwrap();
        assert!(result.input.required_skills.is_empty());
        assert_eq!(result.input.max_fusion_depth, 1);
        assert_eq!(state.max_depth.get_untracked(), 3);
        assert!(untrack(|| state.result_is_stale()));
        assert!(untrack(|| state.can_edit_route()));
        assert_eq!(state.i18n.locale_untracked(), Locale::JaJp);
    });
}

#[test]
fn returning_to_the_original_conditions_clears_the_stale_notice() {
    Owner::new().with(|| {
        let (controller, worker, mut service) = setup();
        let state = controller.state;
        calculate(&controller, &worker, &mut service);
        controller.set_depth(4);
        assert!(untrack(|| state.result_is_stale()));
        controller.set_depth(1);
        assert!(!untrack(|| state.result_is_stale()));
        controller.open_skill_picker(0);
        controller.select_skill(skill_ids::RIBERAMA);
        assert!(untrack(|| state.result_is_stale()));
        controller.remove_skill(0);
        assert!(!untrack(|| state.result_is_stale()));
        controller.set_dlc(DemonContent::DagdaDlc, true);
        assert!(untrack(|| state.result_is_stale()));
        controller.set_dlc(DemonContent::DagdaDlc, false);
        assert!(!untrack(|| state.result_is_stale()));

        let mut result = state.result.get_untracked().unwrap().as_ref().clone();
        result.input.required_skills = vec![SkillId(1), SkillId(2)];
        state.result.set(Some(Arc::new(result)));
        state.required_skills.set(vec![SkillId(2), SkillId(1)]);
        assert!(!untrack(|| state.result_is_stale()));
    });
}

#[test]
fn filtering_during_options_loading_keeps_keyboard_focus_in_the_visible_list() {
    Owner::new().with(|| {
        let (controller, worker, mut service) = setup();
        let state = controller.state;
        calculate(&controller, &worker, &mut service);
        controller.open_options(Vec::new());
        worker.respond(service.handle(request(&worker)));
        let options = state.options.get_untracked().unwrap();
        let choice = options.options.last().unwrap();
        let OptionAcquisitionDto::Fusion { materials, .. } = &choice.acquisition else {
            panic!()
        };
        let query = state.i18n.demon_name_untracked(materials[0].demon);
        controller.select_option(choice.option_id, choice.selected);
        worker.respond(service.handle(request(&worker)));

        for query in [query, "not-a-material".to_owned(), String::new()] {
            controller.open_options(Vec::new());
            let pending = request(&worker);
            let WorkerRequest::GetNodeOptions { request_id, .. } = &pending else {
                panic!()
            };
            state.reveal_options_indicator(*request_id);
            state.option_query.set(query.clone());
            state.option_active_index.set(0);
            worker.respond(service.handle(pending));
            assert_eq!(state.option_query.get_untracked(), query);
            assert!(!state.options_indicator_visible.get_untracked());
            let visible = untrack(|| filtered_options(state));
            let active = untrack(|| source_active_descendant(state));
            if visible.is_empty() {
                assert!(active.is_empty());
                assert!(
                    visible
                        .get(state.option_active_index.get_untracked())
                        .is_none()
                );
            } else {
                let option = &visible[state.option_active_index.get_untracked()];
                assert_eq!(active, format!("source-option-{}", option.option_id));
                assert!(option.selected);
            }
        }
    });
}

#[test]
fn mutation_blocks_new_searches_and_preserves_its_path_when_the_popover_closes() {
    Owner::new().with(|| {
        let (controller, worker, mut service) = setup();
        let state = controller.state;
        calculate(&controller, &worker, &mut service);
        controller.open_options(Vec::new());
        worker.respond(service.handle(request(&worker)));
        let options = state.options.get_untracked().unwrap();
        let choice = options.options.last().unwrap();
        controller.select_option(choice.option_id, choice.selected);
        let pending = request(&worker);
        assert!(state.selection_busy.get_untracked());
        assert!(!untrack(|| state.can_edit_route()));
        assert!(!untrack(|| state.can_search()));
        controller.search();
        controller.open_options(vec![0]);
        controller.reset_default();
        assert!(worker.take_requests().is_empty());
        controller.set_locale(Locale::EnUs);
        assert!(state.panel_path.get_untracked().is_none());
        assert_eq!(state.mutation_path.get_untracked(), Some(Vec::new()));
        worker.respond(service.handle(pending));
        assert!(!state.selection_busy.get_untracked());
        assert!(state.mutation_path.get_untracked().is_none());
        assert!(untrack(|| state.can_edit_route()));
    });
}

#[test]
fn closing_a_mutation_popover_does_not_reset_unrelated_tree_expansion() {
    Owner::new().with(|| {
        let (controller, worker, mut service) = setup();
        let state = controller.state;
        calculate(&controller, &worker, &mut service);
        let result = state.result.get_untracked().unwrap();
        state.active_mutation.set(Some(55));
        state.mutation_path.set(Some(vec![0]));
        state.selection_busy.set(true);
        state.collapsed.set(BTreeSet::from([vec![0, 0], vec![1]]));
        state.close_options();
        worker.respond(WorkerResponse::SelectionChanged {
            request_id: 55,
            snapshot: SelectionSnapshotDto {
                session_id: result.session_id,
                selection_revision: result.selection_revision + 1,
                actual_fusion_depth: result.actual_fusion_depth,
                tree: result.tree.clone().unwrap(),
            },
        });
        assert_eq!(state.collapsed.get_untracked(), BTreeSet::from([vec![1]]));
    });
}

fn setup_with_expansion_tree() -> (Controller, TestWorker) {
    fn node(path: &[u8], children: Vec<RouteTreeNodeDto>) -> RouteTreeNodeDto {
        let acquisition = if children.is_empty() {
            AcquisitionDto::Direct {
                summon_level: 1,
                target_level: 1,
            }
        } else {
            AcquisitionDto::Fusion {
                is_special: false,
                fusion_level: 1,
                target_level: 1,
                materials: children.iter().map(|child| child.demon).collect(),
            }
        };
        RouteTreeNodeDto {
            path: path.to_vec(),
            demon: DemonId(0),
            base_level: 1,
            final_level: 1,
            required_skills: Vec::new(),
            upgrade_skills: Vec::new(),
            acquisition,
            can_change_recipe: true,
            children,
        }
    }

    let (controller, worker, _) = setup();
    let tree = node(
        &[],
        vec![
            node(
                &[0],
                vec![node(&[0, 0], vec![node(&[0, 0, 0], Vec::new())])],
            ),
            node(
                &[1],
                vec![
                    node(&[1, 0], Vec::new()),
                    node(
                        &[1, 1],
                        vec![node(&[1, 1, 0], vec![node(&[1, 1, 0, 0], Vec::new())])],
                    ),
                ],
            ),
        ],
    );
    controller.select_target(Some(tree.demon));
    controller.set_depth(4);
    controller.search();
    let WorkerRequest::Search { request_id, input } = request(&worker) else {
        panic!()
    };
    worker.respond(WorkerResponse::SearchCompleted {
        request_id,
        result: SearchResultDto {
            session_id: 1,
            selection_revision: 0,
            input,
            route_count: "1".to_owned(),
            actual_fusion_depth: 4,
            tree: Some(tree),
        },
    });
    (controller, worker)
}

#[test]
fn changing_a_recipe_expands_its_entire_subtree_and_preserves_other_branches() {
    for (changed, expected) in [
        (vec![1], BTreeSet::from([vec![0]])),
        (vec![1, 1], BTreeSet::from([vec![0]])),
        (vec![1, 1, 0], BTreeSet::from([vec![0]])),
        (vec![], BTreeSet::new()),
    ] {
        Owner::new().with(|| {
            let (controller, worker) = setup_with_expansion_tree();
            let state = controller.state;
            let result = state.result.get_untracked().unwrap();
            let mut tree = result.tree.clone().unwrap();
            let mut collapsed = all_collapsible_paths(&tree);
            collapsed.retain(|path| !changed.starts_with(path) || path == &changed);
            collapsed.remove(&vec![0, 0]);
            state.collapsed.set(collapsed.clone());

            let mut node = &mut tree;
            for &index in &changed {
                node = &mut node.children[usize::from(index)];
            }
            node.children.last_mut().unwrap().demon = DemonId(1);
            let AcquisitionDto::Fusion { materials, .. } = &mut node.acquisition else {
                panic!()
            };
            *materials.last_mut().unwrap() = DemonId(1);

            state.begin_options_request(55, changed.clone());
            controller.select_option(1, false);
            assert_eq!(state.collapsed.get_untracked(), collapsed);
            let WorkerRequest::SelectNodeOption {
                request_id, path, ..
            } = request(&worker)
            else {
                panic!()
            };
            assert_eq!(path, changed);
            assert_eq!(state.mutation_path.get_untracked(), Some(changed.clone()));
            state.close_options();
            worker.respond(WorkerResponse::SelectionChanged {
                request_id,
                snapshot: SelectionSnapshotDto {
                    session_id: result.session_id,
                    selection_revision: result.selection_revision + 1,
                    actual_fusion_depth: 4,
                    tree: tree.clone(),
                },
            });

            assert_eq!(
                state.result.get_untracked().unwrap().tree.as_ref(),
                Some(&tree)
            );
            assert_eq!(
                state.collapsed.get_untracked(),
                expected,
                "path: {changed:?}"
            );
            assert!(state.mutation_path.get_untracked().is_none());
            assert!(untrack(|| state.can_edit_route()));
        });
    }
}

#[test]
fn resetting_the_route_restores_expansion_defaults_from_the_root() {
    Owner::new().with(|| {
        let (controller, worker) = setup_with_expansion_tree();
        let state = controller.state;
        let defaults = BTreeSet::from([vec![0, 0], vec![1, 1], vec![1, 1, 0]]);
        assert_eq!(state.collapsed.get_untracked(), defaults);
        state.collapsed.set(BTreeSet::from([vec![0], vec![1]]));
        controller.reset_default();
        let WorkerRequest::ResetToDefault { request_id, .. } = request(&worker) else {
            panic!()
        };
        let result = state.result.get_untracked().unwrap();
        worker.respond(WorkerResponse::SelectionChanged {
            request_id,
            snapshot: SelectionSnapshotDto {
                session_id: result.session_id,
                selection_revision: result.selection_revision + 1,
                actual_fusion_depth: result.actual_fusion_depth,
                tree: result.tree.clone().unwrap(),
            },
        });
        assert_eq!(state.collapsed.get_untracked(), defaults);
        assert!(untrack(|| state.can_edit_route()));
    });
}

#[test]
fn clearing_a_running_search_terminates_the_old_worker_and_rejects_all_its_callbacks() {
    Owner::new().with(|| {
        let (controller, old_worker, mut old_service) = setup();
        let state = controller.state;
        controller.set_dlc(DemonContent::DagdaDlc, true);
        controller.set_locale(Locale::JaJp);
        controller.search();
        let pending = request(&old_worker);
        let generation = state.worker_generation.get_untracked();
        controller.open_skill_picker(0);
        state.skill_query.set("test".to_owned());
        controller.clear_form();
        assert!(old_worker.terminated());
        assert_eq!(state.worker_generation.get_untracked(), generation + 1);
        assert_eq!(state.worker_status.get_untracked(), WorkerStatus::Loading);
        assert!(!state.searching.get_untracked());
        assert!(!state.skill_picker_open.get_untracked());
        assert!(state.skill_query.get_untracked().is_empty());
        assert!(state.target.get_untracked().is_none());
        assert!(state.result.get_untracked().is_none());
        assert!(state.dagda_dlc.get_untracked());
        assert_eq!(state.i18n.locale_untracked(), Locale::JaJp);

        old_worker.respond(old_service.ready());
        old_worker.fail("late failure");
        let old_response = old_service.handle(pending);
        old_worker.respond(old_response.clone());
        assert_eq!(state.worker_status.get_untracked(), WorkerStatus::Loading);
        assert!(state.error.get_untracked().is_none());
        assert!(state.result.get_untracked().is_none());

        let new_worker = controller.test_worker();
        let mut new_service = WorkerService::new();
        new_worker.respond(new_service.ready());
        controller.select_target(Some(demon_ids::SHIVA));
        state.required_skills.set(Vec::new());
        controller.set_depth(1);
        calculate(&controller, &new_worker, &mut new_service);
        let result = state.result.get_untracked().unwrap();
        old_worker.respond(old_response);
        old_worker.fail("late failure after replacement is ready");
        old_worker.respond(old_service.ready());
        assert!(Arc::ptr_eq(&state.result.get_untracked().unwrap(), &result));
        assert!(state.error.get_untracked().is_none());
        assert!(untrack(|| state.can_edit_route()));
    });
}

#[test]
fn clearing_an_idle_form_keeps_the_ready_worker_and_all_preferences() {
    Owner::new().with(|| {
        let (controller, worker, mut service) = setup();
        let state = controller.state;
        calculate(&controller, &worker, &mut service);
        controller.set_dlc(DemonContent::KonohanaSakuyaDlc, true);
        controller.set_locale(Locale::ZhTw);
        let generation = state.worker_generation.get_untracked();
        controller.clear_form();
        assert!(!worker.terminated());
        assert_eq!(state.worker_generation.get_untracked(), generation);
        assert_eq!(state.worker_status.get_untracked(), WorkerStatus::Ready);
        assert!(state.konohana_sakuya_dlc.get_untracked());
        assert_eq!(state.i18n.locale_untracked(), Locale::ZhTw);
        assert!(!state.session_available.get_untracked());
        assert!(state.result.get_untracked().is_none());
    });
}

#[test]
fn worker_failure_stops_loading_and_dismissal_does_not_lose_the_failed_state() {
    Owner::new().with(|| {
        let (controller, worker, mut service) = setup();
        let state = controller.state;
        calculate(&controller, &worker, &mut service);
        controller.search();
        let pending = request(&worker);
        worker.fail("failed to execute");
        assert_eq!(state.worker_status.get_untracked(), WorkerStatus::Failed);
        assert!(!state.search_indicator_visible.get_untracked());
        assert!(!state.session_available.get_untracked());
        state.error.set(None);
        assert_eq!(state.worker_status.get_untracked(), WorkerStatus::Failed);
        assert!(!untrack(|| state.can_search()));
        assert!(!untrack(|| state.can_edit_route()));
        worker.respond(service.handle(pending));
        assert_eq!(state.worker_status.get_untracked(), WorkerStatus::Failed);
        controller.start_worker();
        assert!(worker.terminated());
        assert_eq!(state.worker_status.get_untracked(), WorkerStatus::Loading);
        controller.test_worker().respond(service.ready());
        assert_eq!(state.worker_status.get_untracked(), WorkerStatus::Ready);
    });
}

#[test]
fn a_global_protocol_failure_does_not_leave_pending_requests_spinning() {
    Owner::new().with(|| {
        let (controller, worker, _) = setup();
        let state = controller.state;
        controller.search();
        request(&worker);
        worker.respond(crate::service::invalid_message_response());
        assert_eq!(state.worker_status.get_untracked(), WorkerStatus::Failed);
        assert!(state.active_search.get_untracked().is_none());
        assert!(!state.searching.get_untracked());
        assert!(!state.search_indicator_visible.get_untracked());
    });
}

#[test]
fn persisted_skills_cannot_be_edited_until_catalog_is_ready() {
    Owner::new().with(|| {
        let state = AppState::new(
            PersistedForm {
                target: Some(demon_ids::SHIVA),
                required_skills: vec![skill_ids::RIBERAMA],
                ..PersistedForm::default()
            },
            I18n::new(Locale::ZhCn),
        );
        let controller = Controller::new(state);
        controller.start_worker();
        assert!(!untrack(|| state.can_edit_skills()));
        controller.open_skill_picker(1);
        controller.remove_skill(0);
        assert!(!state.skill_picker_open.get_untracked());
        assert_eq!(
            state.required_skills.get_untracked(),
            vec![skill_ids::RIBERAMA]
        );
        controller
            .test_worker()
            .respond(WorkerService::new().ready());
        assert!(untrack(|| state.can_edit_skills()));
        controller.open_skill_picker(1);
        assert!(state.skill_picker_open.get_untracked());
        assert_eq!(state.skill_picker_slot.get_untracked(), Some(1));
    });
}

#[test]
fn target_changes_and_dlc_invalidation_close_the_skill_dialog_and_reset_filters() {
    Owner::new().with(|| {
        let (controller, _, _) = setup();
        let state = controller.state;
        controller.open_skill_picker(0);
        state.skill_query.set("test".to_owned());
        controller.select_target(Some(demon_ids::PIXIE));
        assert!(!state.skill_picker_open.get_untracked());
        assert!(state.skill_query.get_untracked().is_empty());

        controller.set_dlc(DemonContent::KonohanaSakuyaDlc, true);
        controller.select_target(Some(demon_ids::KONOHANA_SAKUYA));
        controller.open_skill_picker(state.required_skills.get_untracked().len());
        assert!(state.skill_picker_open.get_untracked());
        controller.set_dlc(DemonContent::KonohanaSakuyaDlc, false);
        assert!(state.target.get_untracked().is_none());
        assert!(!state.skill_picker_open.get_untracked());
        assert!(state.skill_picker_slot.get_untracked().is_none());
        assert!(state.required_skills.get_untracked().is_empty());
    });
}
