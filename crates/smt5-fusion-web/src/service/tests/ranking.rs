use super::*;

fn measure(selection: &RouteSelection) -> (u64, u32, u64) {
    match selection.space.choice(selection.choice_index).unwrap() {
        RouteChoice::Direct(direct) => (0, 0, direct.estimated_macca),
        RouteChoice::Fusion(_) => {
            selection
                .materials
                .iter()
                .fold((1, 1, 0), |(count, depth, macca), child| {
                    let child = measure(child);
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

fn assert_option_metrics(options: &NodeOptionsDto, scores: &[(u64, u32, u64)], outside_macca: u64) {
    assert_eq!(options.options.len(), scores.len());
    for (option, score) in options.options.iter().zip(scores) {
        assert_eq!(
            option.estimated_macca,
            (score.2 - outside_macca).to_string()
        );
        assert!(option.score > 0);
    }
    for (displayed, actual) in options.options.windows(2).zip(scores.windows(2)) {
        assert_eq!(
            displayed[0].score.cmp(&displayed[1].score),
            actual[1].cmp(&actual[0])
        );
    }
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
            let score = measure(&selection);
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
    let scores = visible
        .iter()
        .map(|choice| expected[&choice.key])
        .collect::<Vec<_>>();
    for (choice, expected) in visible.iter().zip(&scores) {
        assert_eq!(score_tuple(&choice.score), *expected);
    }
    assert!(scores.windows(2).all(|pair| pair[0] <= pair[1]));
    assert_option_metrics(&options, &scores, 0);
}

#[test]
fn nested_candidates_are_sorted_by_the_resulting_whole_tree() {
    let mut service = search_service();
    select_deep_root(&mut service);
    let original = service.session.as_ref().unwrap().current.clone().unwrap();
    for child_index in 0..original.materials.len() {
        let path = RoutePath(vec![child_index]);
        let displayed = options(&mut service, &[child_index as u8]);
        let session = service.session.as_ref().unwrap();
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
            let actual = measure(&changed);
            assert_eq!(score_tuple(&candidate.score), actual);
            for (index, other) in original.materials.iter().enumerate() {
                if index != child_index {
                    assert_eq!(&changed.materials[index], other);
                }
            }
            scores.push(actual);
        }
        assert!(scores.windows(2).all(|pair| pair[0] <= pair[1]));
        let outside_macca = measure(&original).2 - measure(&original.materials[child_index]).2;
        assert!(outside_macca > 0);
        assert_option_metrics(&displayed, &scores, outside_macca);
    }
    assert_eq!(
        service.session.as_ref().unwrap().current.as_ref(),
        Some(&original)
    );
}

#[test]
fn displayed_scores_preserve_ties_and_priority_over_macca_without_precision_loss() {
    let mut service = search_service();
    options(&mut service, &[]);
    let choices = &mut service
        .session
        .as_mut()
        .unwrap()
        .options
        .as_mut()
        .unwrap()
        .choices;
    assert_eq!(choices.len(), 3);
    let macca = "18446744073709551616000";
    let better = RouteScore {
        fusion_count: 4_u8.into(),
        fusion_depth: 4,
        estimated_macca: macca.parse().unwrap(),
    };
    choices[0].score = better.clone();
    choices[1].score = better;
    choices[2].score = RouteScore {
        fusion_count: 5_u8.into(),
        fusion_depth: 3,
        estimated_macca: 1_u8.into(),
    };
    let displayed = options(&mut service, &[]);
    assert_eq!(
        displayed
            .options
            .iter()
            .map(|option| option.score)
            .collect::<Vec<_>>(),
        [2, 2, 1]
    );
    assert_eq!(displayed.options[0].estimated_macca, macca);
    assert_eq!(displayed.options[1].estimated_macca, macca);
    assert_eq!(displayed.options[2].estimated_macca, "1");
}

#[test]
fn single_direct_option_has_a_positive_score_and_summoning_cost() {
    let mut service = WorkerService::new();
    service.handle(search_request(demon_ids::PIXIE, &[], 0));
    let displayed = options(&mut service, &[]);
    assert_eq!(displayed.options.len(), 1);
    assert_eq!(displayed.options[0].score, 1);
    let current = service.session.as_ref().unwrap().current.as_ref().unwrap();
    assert_option_metrics(&displayed, &[measure(current)], 0);
}

fn assert_tree_macca(selection: &RouteSelection, tree: &RouteTreeNodeDto) {
    assert_eq!(tree.estimated_macca, measure(selection).2.to_string());
    assert_eq!(tree.children.len(), selection.materials.len());
    for (child, material) in tree.children.iter().zip(&selection.materials) {
        assert_tree_macca(material, child);
    }
    if !tree.children.is_empty() {
        let children_macca = tree
            .children
            .iter()
            .map(|child| child.estimated_macca.parse::<u64>().unwrap())
            .sum::<u64>();
        assert_eq!(tree.estimated_macca, children_macca.to_string());
    }
}

fn tree_nodes(tree: &RouteTreeNodeDto) -> Vec<&RouteTreeNodeDto> {
    let mut nodes = vec![tree];
    for child in &tree.children {
        nodes.extend(tree_nodes(child));
    }
    nodes
}

#[test]
fn direct_element_cost_uses_its_cheapest_normal_material_pair() {
    let mut service = WorkerService::new();
    let response = service.handle(search_request(demon_ids::AEROS, &[], 0));
    let WorkerResponse::SearchCompleted { result, .. } = response else {
        panic!("expected search result")
    };
    let expected = service
        .player_context
        .get_direct_recipes(demon_ids::AEROS)
        .unwrap()
        .iter()
        .filter(|recipe| !recipe.is_special && recipe.materials.len() == 2)
        .map(|recipe| {
            recipe
                .materials
                .iter()
                .map(|material| {
                    u64::from(
                        service
                            .game_data
                            .demons()
                            .get(*material)
                            .unwrap()
                            .compendium_price,
                    )
                })
                .sum::<u64>()
        })
        .min()
        .unwrap();

    assert_eq!(result.tree.unwrap().estimated_macca, expected.to_string());
}

#[test]
fn node_and_recipe_costs_agree_before_and_after_subtree_replacement() {
    let mut service = search_service();
    select_deep_root(&mut service);
    let session = service.session.as_ref().unwrap();
    let current = session.current.as_ref().unwrap();
    let tree = service.tree(&session.selector, current, current, Vec::new());
    assert_tree_macca(current, &tree);
    let root_options = options(&mut service, &[]);
    assert_eq!(
        root_options
            .options
            .iter()
            .find(|option| option.selected)
            .unwrap()
            .estimated_macca,
        tree.estimated_macca
    );
    let mut replacement = None;
    for child in tree_nodes(&tree).into_iter().skip(1) {
        let candidates = options(&mut service, &child.path);
        assert_eq!(
            candidates
                .options
                .iter()
                .find(|option| option.selected)
                .unwrap()
                .estimated_macca,
            child.estimated_macca
        );
        if child.path.len() == 1
            && let Some(candidate) = candidates.options.iter().find(|option| !option.selected)
        {
            replacement = Some((candidates.clone(), candidate.clone()));
        }
    }
    let (candidates, candidate) = replacement.expect("a child must have an alternative recipe");
    let snapshot = select(&mut service, &candidates, candidate.option_id);
    let current = service.session.as_ref().unwrap().current.as_ref().unwrap();
    assert_tree_macca(current, &snapshot.tree);
    let changed = &snapshot.tree.children[candidates.path[0] as usize];
    assert_eq!(changed.estimated_macca, candidate.estimated_macca);
    let updated = options(&mut service, &candidates.path);
    assert_eq!(
        updated
            .options
            .iter()
            .find(|option| option.selected)
            .unwrap()
            .estimated_macca,
        changed.estimated_macca
    );
    let response = service.handle(WorkerRequest::ResetToDefault {
        request_id: 4,
        session_id: snapshot.session_id,
        selection_revision: snapshot.selection_revision,
    });
    let WorkerResponse::SelectionChanged { snapshot, .. } = response else {
        panic!("expected the default route");
    };
    assert_tree_macca(
        service.session.as_ref().unwrap().current.as_ref().unwrap(),
        &snapshot.tree,
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
        measure(service.session.as_ref().unwrap().current.as_ref().unwrap()),
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
            if measure(&changed).1 < deep.space.fusion_depth {
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
    assert!(measure(&edited).1 < edited.space.fusion_depth);
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
    assert_eq!(score_tuple(&current_option.score), measure(&edited));
    assert_eq!(
        root_options
            .options
            .iter()
            .find(|option| option.selected)
            .unwrap()
            .estimated_macca,
        measure(&edited).2.to_string()
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
