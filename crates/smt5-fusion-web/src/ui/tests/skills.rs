use super::*;

use smt5_fusion_core::dataset::demon_ids;

use super::super::{
    selectors::{demon, skill_eligible_for_target},
    state::{SKILL_CAPACITY, WorkerStatus},
};

fn setup() -> (Controller, Vec<SkillId>) {
    let state = AppState::new(PersistedForm::default(), I18n::new(Locale::ZhCn));
    state.handle_response(WorkerService::new().ready());
    let controller = Controller::new(state);
    controller.select_target(Some(demon_ids::SHIVA));
    state.required_skills.set(Vec::new());
    let choices = untrack(|| filtered_skills(state))
        .into_iter()
        .map(|skill| skill.id)
        .collect::<Vec<_>>();
    assert!(choices.len() > SKILL_CAPACITY);
    (controller, choices)
}

#[test]
fn opening_either_modal_closes_the_other_and_resets_its_filters() {
    Owner::new().with(|| {
        let (controller, _) = setup();
        let state = controller.state;
        assert!(!untrack(|| state.modal_open()));
        state.target_picker_open.set(true);
        controller.open_skill_picker(0);
        assert!(untrack(|| state.modal_open()));
        assert!(!state.target_picker_open.get_untracked());
        state.skill_query.set("test".to_owned());
        state
            .skill_category
            .set(Some(crate::protocol::SkillCategory::Fire));

        state.begin_options_request(7, vec![0]);
        assert!(untrack(|| state.modal_open()));
        assert!(!state.skill_picker_open.get_untracked());
        assert!(state.skill_picker_slot.get_untracked().is_none());
        assert!(state.skill_query.get_untracked().is_empty());
        assert!(state.skill_category.get_untracked().is_none());
        state.option_query.set("test".to_owned());

        controller.open_skill_picker(0);
        assert!(untrack(|| state.modal_open()));
        assert!(state.panel_path.get_untracked().is_none());
        assert!(state.active_options.get_untracked().is_none());
        assert!(state.option_query.get_untracked().is_empty());
        state.close_skill_picker();
        assert!(!untrack(|| state.modal_open()));

        state.begin_options_request(8, vec![]);
        assert!(untrack(|| state.modal_open()));
        state.close_options();
        assert!(!untrack(|| state.modal_open()));
    });
}

#[test]
fn filled_skill_slots_replace_in_place_including_at_capacity() {
    Owner::new().with(|| {
        let (controller, choices) = setup();
        let state = controller.state;
        for count in [1, 3, SKILL_CAPACITY] {
            let original = choices[..count].to_vec();
            for slot in 0..count {
                state.required_skills.set(original.clone());
                controller.open_skill_picker(slot);
                assert!(state.skill_picker_open.get_untracked());
                assert_eq!(state.skill_picker_slot.get_untracked(), Some(slot));
                assert_eq!(state.required_skills.get_untracked(), original);

                controller.select_skill(choices[count]);
                let mut expected = original.clone();
                expected[slot] = choices[count];
                assert_eq!(state.required_skills.get_untracked(), expected);
                assert_eq!(state.current_input().unwrap().required_skills, expected);
                assert!(!state.skill_picker_open.get_untracked());
                assert!(state.skill_picker_slot.get_untracked().is_none());
            }
        }
    });
}

#[test]
fn empty_skill_slots_still_append_without_creating_gaps() {
    Owner::new().with(|| {
        let (controller, choices) = setup();
        let state = controller.state;
        for slot in [1, SKILL_CAPACITY - 1] {
            state.required_skills.set(vec![choices[0]]);
            controller.open_skill_picker(slot);
            controller.select_skill(choices[1]);
            assert_eq!(state.required_skills.get_untracked(), choices[..2]);
            assert!(!state.skill_picker_open.get_untracked());
        }
    });
}

#[test]
fn closing_the_skill_picker_preserves_skills_and_resets_filters() {
    Owner::new().with(|| {
        let (controller, choices) = setup();
        let state = controller.state;
        state.required_skills.set(choices[..2].to_vec());
        let original = state.current_input();
        for slot in [0, 1, SKILL_CAPACITY - 1] {
            controller.open_skill_picker(slot);
            state.skill_query.set("test".to_owned());
            state
                .skill_category
                .set(Some(crate::protocol::SkillCategory::Fire));
            state.close_skill_picker();
            assert_eq!(state.current_input(), original);
            assert!(!state.skill_picker_open.get_untracked());
            assert!(state.skill_picker_slot.get_untracked().is_none());
            assert!(state.skill_query.get_untracked().is_empty());
            assert!(state.skill_category.get_untracked().is_none());
        }
    });
}

#[test]
fn skill_replacement_allows_the_current_skill_but_not_other_slots_skills() {
    Owner::new().with(|| {
        let (controller, choices) = setup();
        let state = controller.state;
        state.required_skills.set(choices[..2].to_vec());
        controller.open_skill_picker(0);
        let visible = untrack(|| filtered_skills(state));
        assert!(visible.iter().any(|skill| skill.id == choices[0]));
        assert!(!visible.iter().any(|skill| skill.id == choices[1]));

        controller.select_skill(choices[1]);
        assert_eq!(state.required_skills.get_untracked(), choices[..2]);
        assert!(state.skill_picker_open.get_untracked());

        controller.select_skill(choices[0]);
        assert_eq!(state.required_skills.get_untracked(), choices[..2]);
        assert!(!state.skill_picker_open.get_untracked());

        controller.open_skill_picker(2);
        let visible = untrack(|| filtered_skills(state));
        assert!(!visible.iter().any(|skill| choices[..2].contains(&skill.id)));
        controller.select_skill(choices[0]);
        assert_eq!(state.required_skills.get_untracked(), choices[..2]);
        assert!(state.skill_picker_open.get_untracked());
    });
}

#[test]
fn skill_replacement_preserves_eligibility_and_search_filters() {
    Owner::new().with(|| {
        let (controller, choices) = setup();
        let state = controller.state;
        state.required_skills.set(vec![choices[0]]);
        controller.open_skill_picker(0);
        let catalog = state.catalog.get_untracked().unwrap();
        let target = demon(&catalog, demon_ids::SHIVA).unwrap();
        let invalid = catalog
            .skills
            .iter()
            .filter(|skill| !skill_eligible_for_target(target, skill))
            .map(|skill| skill.id)
            .collect::<Vec<_>>();
        assert!(!invalid.is_empty());
        for id in invalid.into_iter().chain([SkillId(u32::MAX)]) {
            controller.select_skill(id);
            assert_eq!(state.required_skills.get_untracked(), vec![choices[0]]);
            assert!(state.skill_picker_open.get_untracked());
        }
        let current = untrack(|| filtered_skills(state))
            .into_iter()
            .find(|skill| skill.id == choices[0])
            .unwrap();
        state.skill_category.set(Some(current.category));
        state
            .skill_query
            .set(untrack(|| state.i18n.skill_name(current.id)));
        let visible = untrack(|| filtered_skills(state));
        assert!(visible.iter().any(|skill| skill.id == current.id));
        assert!(
            visible
                .iter()
                .all(|skill| skill.category == current.category)
        );
    });
}

#[test]
fn replacing_a_skill_updates_the_form_without_changing_the_result_snapshot() {
    Owner::new().with(|| {
        let (controller, choices) = setup();
        let state = controller.state;
        state.required_skills.set(choices[..2].to_vec());
        let result = Arc::new(SearchResultDto {
            session_id: 1,
            selection_revision: 0,
            input: state.current_input().unwrap(),
            route_count: "0".to_owned(),
            actual_fusion_depth: 0,
            tree: None,
        });
        state.result.set(Some(result.clone()));
        assert!(!untrack(|| state.result_is_stale()));
        controller.open_skill_picker(0);
        controller.select_skill(choices[2]);
        assert!(untrack(|| state.result_is_stale()));
        assert!(Arc::ptr_eq(&state.result.get_untracked().unwrap(), &result));
        assert_eq!(result.input.required_skills, choices[..2]);
        assert_eq!(
            state.current_input().unwrap().required_skills,
            vec![choices[2], choices[1]]
        );
        controller.open_skill_picker(0);
        controller.select_skill(choices[0]);
        assert!(!untrack(|| state.result_is_stale()));
        assert!(Arc::ptr_eq(&state.result.get_untracked().unwrap(), &result));
    });
}

#[test]
fn removing_a_skill_dismisses_the_picker_before_slot_indices_shift() {
    Owner::new().with(|| {
        let (controller, choices) = setup();
        let state = controller.state;
        state.required_skills.set(choices[..3].to_vec());
        controller.open_skill_picker(1);
        controller.remove_skill(0);
        assert_eq!(state.required_skills.get_untracked(), choices[1..3]);
        assert!(!state.skill_picker_open.get_untracked());
        assert!(state.skill_picker_slot.get_untracked().is_none());
        controller.select_skill(choices[3]);
        assert_eq!(state.required_skills.get_untracked(), choices[1..3]);
        controller.open_skill_picker(1);
        controller.select_skill(choices[3]);
        assert_eq!(
            state.required_skills.get_untracked(),
            vec![choices[1], choices[3]]
        );
    });
}

#[test]
fn skill_selection_requires_an_editable_form_and_a_valid_open_slot() {
    Owner::new().with(|| {
        let (controller, choices) = setup();
        let state = controller.state;
        state.required_skills.set(vec![choices[0]]);
        for slot in [SKILL_CAPACITY, usize::MAX] {
            controller.open_skill_picker(slot);
            assert!(!state.skill_picker_open.get_untracked());
            controller.select_skill(choices[1]);
            assert_eq!(state.required_skills.get_untracked(), vec![choices[0]]);
        }
        for status in [WorkerStatus::Loading, WorkerStatus::Failed] {
            state.worker_status.set(status);
            controller.open_skill_picker(0);
            controller.select_skill(choices[1]);
            controller.remove_skill(0);
            assert!(!state.skill_picker_open.get_untracked());
            assert_eq!(state.required_skills.get_untracked(), vec![choices[0]]);
        }
        state.worker_status.set(WorkerStatus::Ready);
        controller.select_target(None);
        controller.open_skill_picker(0);
        controller.select_skill(choices[1]);
        assert!(!state.skill_picker_open.get_untracked());
        assert!(state.required_skills.get_untracked().is_empty());
    });
}
