use std::{collections::BTreeSet, sync::Arc};

use leptos::prelude::*;

use crate::protocol::{
    AcquisitionDto, DemonId, DlcSettingsDto, OptionAcquisitionDto, RouteTreeNodeDto,
    SearchInputDto, SearchResultDto, VisibleOptionDto,
};

use super::{
    selectors::{all_collapsible_paths, grouped_digits, option_matches},
    state::{AppState, Controller, PersistedForm},
};

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

    assert!(option_matches(&direct, ""));
    assert!(!option_matches(&direct, "巴隆"));
    assert!(option_matches(&fusion, "巴隆"));
    assert!(!option_matches(&fusion, "湿婆"));
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
fn selected_depth_is_copied_into_the_search_input() {
    Owner::new().with(|| {
        let state = AppState::new(PersistedForm::default());
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
        let state = AppState::new(PersistedForm::default());
        state.target.set(Some(DemonId(193)));
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
        state.dirty.set(true);

        Controller::new(state).clear_form();

        assert!(state.target.get_untracked().is_none());
        assert!(state.result.get_untracked().is_none());
        assert!(state.active_search.get_untracked().is_none());
        assert!(!state.searching.get_untracked());
        assert!(!state.dirty.get_untracked());
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
