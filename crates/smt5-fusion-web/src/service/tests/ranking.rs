use super::*;

fn measure(data: &GameData, selection: &RouteSelection) -> (u64, u32, u64) {
    match selection.space.choice(selection.choice_index).unwrap() {
        RouteChoice::Direct(_) => (
            0,
            0,
            u64::from(
                data.demons()
                    .get(selection.space.demon)
                    .unwrap()
                    .compendium_price,
            ),
        ),
        RouteChoice::Fusion(_) => {
            selection
                .materials
                .iter()
                .fold((1, 1, 0), |(count, depth, macca), child| {
                    let child = measure(data, child);
                    (count + child.0, depth.max(child.1 + 1), macca + child.2)
                })
        }
    }
}

fn score_tuple(score: &RouteScore) -> (u64, u32, u64) {
    (
        (&score.fusion_count).try_into().unwrap(),
        score.fusion_depth,
        (&score.estimated_macca).try_into().unwrap(),
    )
}

fn search_service() -> WorkerService {
    let mut service = WorkerService::new();
    assert!(matches!(
        service.handle(search_request(demon_ids::SHIVA, &[skill_ids::RIBERAMA], 4)),
        WorkerResponse::SearchCompleted { .. }
    ));
    service
}

fn options(service: &mut WorkerService, path: &[u8]) -> NodeOptionsDto {
    let session = service.session.as_ref().unwrap();
    let response = service.handle(WorkerRequest::GetNodeOptions {
        request_id: 2,
        session_id: session.id,
        selection_revision: session.revision,
        path: path.to_vec(),
    });
    let WorkerResponse::NodeOptions { options, .. } = response else {
        panic!("expected options")
    };
    options
}

fn select(
    service: &mut WorkerService,
    options: &NodeOptionsDto,
    option_id: u32,
) -> SelectionSnapshotDto {
    let response = service.handle(WorkerRequest::SelectNodeOption {
        request_id: 3,
        session_id: options.session_id,
        selection_revision: options.selection_revision,
        path: options.path.clone(),
        option_id,
    });
    let WorkerResponse::SelectionChanged { snapshot, .. } = response else {
        panic!("expected selection")
    };
    snapshot
}

fn select_deep_root(service: &mut WorkerService) {
    let options = options(service, &[]);
    let deep = options
        .options
        .iter()
        .find(|option| {
            matches!(
                option.acquisition,
                OptionAcquisitionDto::Fusion { route_depth: 4, .. }
            )
        })
        .unwrap();
    select(service, &options, deep.option_id);
}

#[test]
fn every_recipe_group_uses_its_best_skill_assignment() {
    let mut service = WorkerService::new();
    service.handle(search_request(
        demon_ids::PIXIE,
        &[skill_ids::AGI, skill_ids::BUFU],
        2,
    ));
    let options = options(&mut service, &[]);
    assert!(options.options.first().unwrap().selected);
    let session = service.session.as_ref().unwrap();
    let current = session.current.as_ref().unwrap();
    let visible = &session.options.as_ref().unwrap().choices;
    let mut expected = HashMap::new();
    let mut raw_count = 0;
    for space in &session.selector.routes {
        for (index, choice) in space.choices.iter().enumerate() {
            let selection = current
                .select_choice(
                    &service.game_data,
                    &service.player_context,
                    &RoutePath(Vec::new()),
                    Rc::clone(space),
                    index,
                )
                .unwrap()
                .new_subtree;
            let score = measure(&service.game_data, &selection);
            let key = choice_key(space.fusion_depth, choice);
            expected
                .entry(key)
                .and_modify(|best: &mut (u64, u32, u64)| *best = (*best).min(score))
                .or_insert(score);
            raw_count += 1;
        }
    }
    assert!(raw_count > visible.len());
    assert_eq!(expected.len(), visible.len());
    for choice in visible {
        assert_eq!(score_tuple(&choice.score), expected[&choice.key]);
    }
    assert!(
        visible
            .windows(2)
            .all(|pair| pair[0].score <= pair[1].score)
    );
}

#[test]
fn nested_candidates_are_sorted_by_the_resulting_whole_tree() {
    let mut service = search_service();
    select_deep_root(&mut service);
    let original = service.session.as_ref().unwrap().current.clone().unwrap();
    for child_index in 0..original.materials.len() {
        let path = RoutePath(vec![child_index]);
        options(&mut service, &[child_index as u8]);
        let session = service.session.as_ref().unwrap();
        let context = ReplacementContext::new(&service.game_data, &original, &path).unwrap();
        let mut scores = Vec::new();
        for candidate in &session.options.as_ref().unwrap().choices {
            let mut changed = original.clone();
            if !candidate.selected {
                let update = original
                    .select_choice(
                        &service.game_data,
                        &service.player_context,
                        &path,
                        Rc::clone(&candidate.space),
                        candidate.choice_index,
                    )
                    .unwrap();
                changed.apply_update(update).unwrap();
            }
            let actual = measure(&service.game_data, &changed);
            assert_eq!(score_tuple(&context.score(&candidate.score)), actual);
            for (index, other) in original.materials.iter().enumerate() {
                if index != child_index {
                    assert_eq!(&changed.materials[index], other);
                }
            }
            scores.push(actual);
        }
        assert!(scores.windows(2).all(|pair| pair[0] <= pair[1]));
    }
    assert_eq!(
        service.session.as_ref().unwrap().current.as_ref(),
        Some(&original)
    );
}

#[test]
fn another_branch_can_make_a_deeper_cheaper_candidate_preferable() {
    let context = ReplacementContext {
        outside: RouteScore {
            fusion_count: 5_u8.into(),
            fusion_depth: 4,
            estimated_macca: 1_000_u32.into(),
        },
        path_depth: 1,
    };
    let shallow = RouteScore {
        fusion_count: 3_u8.into(),
        fusion_depth: 2,
        estimated_macca: 100_u8.into(),
    };
    let deep = RouteScore {
        fusion_count: 3_u8.into(),
        fusion_depth: 3,
        estimated_macca: 10_u8.into(),
    };
    assert!(shallow < deep);
    assert!(context.score(&deep) < context.score(&shallow));
}

#[test]
fn cached_options_keep_ids_stable_and_old_revisions_are_rejected() {
    let mut service = search_service();
    let first = options(&mut service, &[]);
    let cached = service
        .session
        .as_ref()
        .unwrap()
        .options
        .as_ref()
        .unwrap()
        .choices
        .as_ptr();
    assert_eq!(options(&mut service, &[]), first);
    assert_eq!(
        service
            .session
            .as_ref()
            .unwrap()
            .options
            .as_ref()
            .unwrap()
            .choices
            .as_ptr(),
        cached
    );
    options(&mut service, &[0]);
    assert_eq!(options(&mut service, &[]), first);
    let replacement = first
        .options
        .iter()
        .find(|option| !option.selected)
        .unwrap();
    let cached_choice = &service
        .session
        .as_ref()
        .unwrap()
        .options
        .as_ref()
        .unwrap()
        .choices[replacement.option_id as usize];
    let expected = score_tuple(&cached_choice.score);
    let selected = select(&mut service, &first, replacement.option_id);
    assert_eq!(
        measure(
            &service.game_data,
            service.session.as_ref().unwrap().current.as_ref().unwrap()
        ),
        expected
    );
    assert_eq!(selected.selection_revision, first.selection_revision + 1);
    assert!(service.session.as_ref().unwrap().options.is_none());
    let old = service.handle(WorkerRequest::SelectNodeOption {
        request_id: 4,
        session_id: first.session_id,
        selection_revision: first.selection_revision,
        path: first.path.clone(),
        option_id: replacement.option_id,
    });
    assert!(matches!(
        old,
        WorkerResponse::Failure {
            failure: WorkerFailureDto {
                code: WorkerFailureCode::StaleSelection,
                ..
            },
            ..
        }
    ));
}

#[test]
fn current_options_preserve_manual_edits_and_reset_restores_the_ranked_default() {
    let mut service = search_service();
    let default = service.session.as_ref().unwrap().current.clone().unwrap();
    select_deep_root(&mut service);
    let deep = service.session.as_ref().unwrap().current.clone().unwrap();
    let mut replacement = None;
    for index in 0..deep.materials.len() {
        let options = options(&mut service, &[index as u8]);
        let session = service.session.as_ref().unwrap();
        for (option_index, candidate) in
            session.options.as_ref().unwrap().choices.iter().enumerate()
        {
            if candidate.selected {
                continue;
            }
            let mut changed = deep.clone();
            let update = deep
                .select_choice(
                    &service.game_data,
                    &service.player_context,
                    &RoutePath(vec![index]),
                    Rc::clone(&candidate.space),
                    candidate.choice_index,
                )
                .unwrap();
            changed.apply_update(update).unwrap();
            if measure(&service.game_data, &changed).1 < deep.space.fusion_depth {
                replacement = Some((options.clone(), option_index as u32));
                break;
            }
        }
        if replacement.is_some() {
            break;
        }
    }
    let (child_options, option_id) = replacement.expect("a shallower child must be available");
    select(&mut service, &child_options, option_id);
    let edited = service.session.as_ref().unwrap().current.clone().unwrap();
    assert!(measure(&service.game_data, &edited).1 < edited.space.fusion_depth);
    let root_options = options(&mut service, &[]);
    assert_eq!(
        service.session.as_ref().unwrap().current.as_ref(),
        Some(&edited)
    );
    let session = service.session.as_ref().unwrap();
    let current_option = session
        .options
        .as_ref()
        .unwrap()
        .choices
        .iter()
        .find(|choice| choice.selected)
        .unwrap();
    assert_eq!(
        score_tuple(&current_option.score),
        measure(&service.game_data, &edited)
    );
    let current_id = root_options
        .options
        .iter()
        .find(|option| option.selected)
        .unwrap()
        .option_id;
    let unchanged = select(&mut service, &root_options, current_id);
    assert_eq!(
        unchanged.selection_revision,
        root_options.selection_revision
    );
    assert_eq!(
        service.session.as_ref().unwrap().current.as_ref(),
        Some(&edited)
    );
    let reset = service.handle(WorkerRequest::ResetToDefault {
        request_id: 5,
        session_id: root_options.session_id,
        selection_revision: unchanged.selection_revision,
    });
    assert!(matches!(reset, WorkerResponse::SelectionChanged { .. }));
    assert_eq!(
        service.session.as_ref().unwrap().current.as_ref(),
        Some(&default)
    );
    assert!(service.session.as_ref().unwrap().options.is_none());
}
