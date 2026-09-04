use std::{collections::HashSet, rc::Rc};

use smt5_fusion_core::{
    data::game_data::GameData,
    dataset,
    model::{
        demon::{DemonId, SkillAcquisition},
        player_context::PlayerContext,
        route_space::{
            RouteChoice, RoutePath, RouteSelection, RouteSelector, RouteSpace, SelectionError,
        },
        skill::SkillCategory,
    },
    reverse_search::{self, SearchError, SearchRequest, SearchSafetyLimit},
};

use crate::protocol::{
    AcquisitionDto, CatalogDto, DemonCatalogDto, MAX_FUSION_DEPTH, NodeOptionsDto,
    OptionAcquisitionDto, OptionMaterialDto, RouteTreeNodeDto, SearchInputDto, SearchResultDto,
    SelectionSnapshotDto, SkillCatalogDto, UpgradeSkillDto, VisibleOptionDto, WorkerFailureCode,
    WorkerFailureDto, WorkerRequest, WorkerResponse,
};

pub struct WorkerService {
    game_data: GameData,
    player_context: PlayerContext,
    next_session_id: u64,
    session: Option<SearchSession>,
}

struct SearchSession {
    id: u64,
    revision: u64,
    selector: RouteSelector,
    current: Option<RouteSelection>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum VisibleChoiceKey {
    Direct,
    Fusion {
        route_depth: u32,
        materials: Vec<DemonId>,
    },
}

struct VisibleChoice {
    key: VisibleChoiceKey,
    space: Rc<RouteSpace>,
    choice_index: usize,
}

impl Default for WorkerService {
    fn default() -> Self {
        Self::new()
    }
}

impl WorkerService {
    pub fn new() -> Self {
        Self {
            game_data: dataset::game_data(),
            player_context: PlayerContext::default(),
            next_session_id: 1,
            session: None,
        }
    }

    pub fn ready(&self) -> WorkerResponse {
        WorkerResponse::Ready {
            catalog: self.catalog(),
        }
    }

    pub fn handle(&mut self, request: WorkerRequest) -> WorkerResponse {
        match request {
            WorkerRequest::Search { request_id, input } => self.search(request_id, input),
            WorkerRequest::GetNodeOptions {
                request_id,
                session_id,
                selection_revision,
                path,
            } => self.node_options(request_id, session_id, selection_revision, path),
            WorkerRequest::SelectNodeOption {
                request_id,
                session_id,
                selection_revision,
                path,
                option_id,
            } => self.select_option(request_id, session_id, selection_revision, path, option_id),
            WorkerRequest::ResetToDefault {
                request_id,
                session_id,
                selection_revision,
            } => self.reset(request_id, session_id, selection_revision),
        }
    }

    fn catalog(&self) -> CatalogDto {
        let demons = self
            .game_data
            .demons()
            .iter()
            .map(|demon| {
                let mut initial_skills = demon
                    .natural_skills
                    .iter()
                    .filter_map(|natural| match natural.acquisition {
                        SkillAcquisition::Initial { order } => Some((order, natural.skill)),
                        SkillAcquisition::Level { .. } => None,
                    })
                    .collect::<Vec<_>>();
                initial_skills.sort_unstable_by_key(|&(order, _)| order);
                let initial_skills = initial_skills.into_iter().map(|(_, skill)| skill).collect();
                DemonCatalogDto {
                    id: demon.id,
                    race: demon.race,
                    base_level: demon.base_level,
                    content: demon.content,
                    initial_skills,
                    natural_skills: demon
                        .natural_skills
                        .iter()
                        .map(|natural| natural.skill)
                        .collect(),
                }
            })
            .collect();
        let skills = self
            .game_data
            .skills()
            .iter()
            .map(|skill| SkillCatalogDto {
                id: skill.id,
                category: skill.category,
                inheritable: skill.inheritable,
                supported: skill.category != SkillCategory::Innate
                    && !skill.flags.magatsuhi
                    && !skill.flags.item_only,
            })
            .collect();
        CatalogDto { demons, skills }
    }

    fn search(&mut self, request_id: u64, input: SearchInputDto) -> WorkerResponse {
        self.session = None;
        if input.max_fusion_depth > MAX_FUSION_DEPTH {
            return failure(request_id, WorkerFailureCode::InvalidMessage);
        }

        self.player_context
            .set_konohana_sakuya_dlc(input.dlc.konohana_sakuya);
        self.player_context.set_dagda_dlc(input.dlc.dagda);
        self.player_context.prepare_direct_recipes(&self.game_data);

        let request = SearchRequest {
            target: input.target,
            required_skills: input.required_skills.clone(),
            max_fusion_depth: input.max_fusion_depth,
        };
        let selector = match reverse_search::search(&self.game_data, &self.player_context, &request)
        {
            Ok(selector) => selector,
            Err(error) => return search_failure(request_id, error),
        };
        let session_id = self.next_session_id;
        self.next_session_id = self.next_session_id.wrapping_add(1).max(1);
        let current = selector.default_selection.clone();
        let result = SearchResultDto {
            session_id,
            selection_revision: 0,
            input,
            route_count: selector.route_count.to_string(),
            actual_fusion_depth: current.as_ref().map_or(0, actual_depth),
            tree: current
                .as_ref()
                .map(|selection| self.tree(selection, Vec::new())),
        };
        self.session = Some(SearchSession {
            id: session_id,
            revision: 0,
            selector,
            current,
        });
        WorkerResponse::SearchCompleted { request_id, result }
    }

    fn node_options(
        &self,
        request_id: u64,
        session_id: u64,
        selection_revision: u64,
        path: Vec<u8>,
    ) -> WorkerResponse {
        let session = match self.session(session_id, selection_revision) {
            Ok(session) => session,
            Err(code) => return failure(request_id, code),
        };
        let Some(current) = session.current.as_ref() else {
            return failure(request_id, WorkerFailureCode::NoRoute);
        };
        let route_path = route_path(&path);
        let selected = match selection_at(current, &route_path) {
            Some(selected) => selected,
            None => return failure(request_id, WorkerFailureCode::InvalidPath),
        };
        let spaces = match spaces_for_path(&session.selector, current, &route_path) {
            Ok(spaces) => spaces,
            Err(_) => return failure(request_id, WorkerFailureCode::InvalidPath),
        };
        let choices = visible_choices(spaces);
        let selected_key = selected_key(selected);
        let options = choices
            .iter()
            .enumerate()
            .map(|(index, visible)| VisibleOptionDto {
                option_id: u32::try_from(index).expect("visible option count must fit in u32"),
                selected: selected_key.as_ref() == Some(&visible.key),
                acquisition: self.option_acquisition(visible),
            })
            .collect();
        WorkerResponse::NodeOptions {
            request_id,
            options: NodeOptionsDto {
                session_id,
                selection_revision,
                path,
                demon: selected.space.demon,
                options,
            },
        }
    }

    fn select_option(
        &mut self,
        request_id: u64,
        session_id: u64,
        selection_revision: u64,
        path: Vec<u8>,
        option_id: u32,
    ) -> WorkerResponse {
        let route_path = route_path(&path);
        let (space, choice_index, unchanged) = {
            let session = match self.session(session_id, selection_revision) {
                Ok(session) => session,
                Err(code) => return failure(request_id, code),
            };
            let Some(current) = session.current.as_ref() else {
                return failure(request_id, WorkerFailureCode::NoRoute);
            };
            let spaces = match spaces_for_path(&session.selector, current, &route_path) {
                Ok(spaces) => spaces,
                Err(_) => return failure(request_id, WorkerFailureCode::InvalidPath),
            };
            let choices = visible_choices(spaces);
            let Some(visible) = choices.get(option_id as usize) else {
                return failure(request_id, WorkerFailureCode::InvalidOption);
            };
            let unchanged = selection_at(current, &route_path)
                .and_then(selected_key)
                .is_some_and(|selected| selected == visible.key);
            (Rc::clone(&visible.space), visible.choice_index, unchanged)
        };

        if unchanged {
            let session = self.session.as_ref().expect("validated session must exist");
            return self.selection_changed(request_id, session);
        }

        let update = {
            let session = self.session.as_ref().expect("validated session must exist");
            let current = session
                .current
                .as_ref()
                .expect("validated selection must exist");
            match current.select_choice(
                &self.game_data,
                &self.player_context,
                &route_path,
                space,
                choice_index,
            ) {
                Ok(update) => update,
                Err(error) => return selection_failure(request_id, error),
            }
        };
        {
            let session = self.session.as_mut().expect("validated session must exist");
            let current = session
                .current
                .as_mut()
                .expect("validated selection must exist");
            if let Err(error) = current.apply_update(update) {
                return selection_failure(request_id, error);
            }
            session.revision = session.revision.wrapping_add(1);
        }
        let session = self.session.as_ref().expect("updated session must exist");
        self.selection_changed(request_id, session)
    }

    fn reset(
        &mut self,
        request_id: u64,
        session_id: u64,
        selection_revision: u64,
    ) -> WorkerResponse {
        if let Err(code) = self.session(session_id, selection_revision) {
            return failure(request_id, code);
        }
        {
            let session = self.session.as_mut().expect("validated session must exist");
            let Some(default) = session.selector.default_selection.clone() else {
                return failure(request_id, WorkerFailureCode::NoRoute);
            };
            session.current = Some(default);
            session.revision = session.revision.wrapping_add(1);
        }
        let session = self.session.as_ref().expect("reset session must exist");
        self.selection_changed(request_id, session)
    }

    fn session(
        &self,
        session_id: u64,
        selection_revision: u64,
    ) -> Result<&SearchSession, WorkerFailureCode> {
        let session = self
            .session
            .as_ref()
            .ok_or(WorkerFailureCode::InvalidSession)?;
        if session.id != session_id {
            return Err(WorkerFailureCode::InvalidSession);
        }
        if session.revision != selection_revision {
            return Err(WorkerFailureCode::StaleSelection);
        }
        Ok(session)
    }

    fn selection_changed(&self, request_id: u64, session: &SearchSession) -> WorkerResponse {
        let current = session
            .current
            .as_ref()
            .expect("selection response requires current route");
        WorkerResponse::SelectionChanged {
            request_id,
            snapshot: SelectionSnapshotDto {
                session_id: session.id,
                selection_revision: session.revision,
                actual_fusion_depth: actual_depth(current),
                tree: self.tree(current, Vec::new()),
            },
        }
    }

    fn tree(&self, selection: &RouteSelection, path: Vec<u8>) -> RouteTreeNodeDto {
        let meta = self
            .game_data
            .demons()
            .get(selection.space.demon)
            .expect("selection demon must exist");
        let choice = selection
            .space
            .choice(selection.choice_index)
            .expect("selection choice must exist");
        let acquisition = match choice {
            RouteChoice::Direct(direct) => AcquisitionDto::Direct {
                summon_level: meta.base_level,
                target_level: direct.target_level,
            },
            RouteChoice::Fusion(fusion) => AcquisitionDto::Fusion {
                is_special: fusion.recipe.is_special,
                fusion_level: meta.base_level,
                target_level: fusion.target_level,
                materials: fusion.recipe.materials.clone(),
            },
        };
        let upgrade_skills = meta
            .natural_skills
            .iter()
            .filter_map(|natural| match natural.acquisition {
                SkillAcquisition::Level { level }
                    if level > meta.base_level
                        && level <= selection.demon.level
                        && selection.space.required_skills.contains(&natural.skill) =>
                {
                    Some(UpgradeSkillDto {
                        skill: natural.skill,
                        level,
                    })
                }
                SkillAcquisition::Initial { .. } | SkillAcquisition::Level { .. } => None,
            })
            .collect();
        let children = selection
            .materials
            .iter()
            .enumerate()
            .map(|(index, child)| {
                let mut child_path = path.clone();
                child_path.push(u8::try_from(index).expect("material index must fit in u8"));
                self.tree(child, child_path)
            })
            .collect();
        RouteTreeNodeDto {
            path,
            demon: selection.space.demon,
            base_level: meta.base_level,
            final_level: selection.demon.level,
            required_skills: selection.space.required_skills.clone(),
            upgrade_skills,
            acquisition,
            children,
        }
    }

    fn option_acquisition(&self, visible: &VisibleChoice) -> OptionAcquisitionDto {
        let meta = self
            .game_data
            .demons()
            .get(visible.space.demon)
            .expect("choice demon must exist");
        match visible
            .space
            .choice(visible.choice_index)
            .expect("visible choice must exist")
        {
            RouteChoice::Direct(direct) => OptionAcquisitionDto::Direct {
                summon_level: meta.base_level,
                target_level: direct.target_level,
            },
            RouteChoice::Fusion(fusion) => OptionAcquisitionDto::Fusion {
                is_special: fusion.recipe.is_special,
                route_depth: visible.space.fusion_depth,
                fusion_level: meta.base_level,
                target_level: fusion.target_level,
                materials: fusion
                    .recipe
                    .materials
                    .iter()
                    .zip(&fusion.materials)
                    .map(|(&demon, material)| OptionMaterialDto {
                        demon,
                        required_skills: material.required_skills.clone(),
                    })
                    .collect(),
            },
        }
    }
}

fn spaces_for_path(
    selector: &RouteSelector,
    current: &RouteSelection,
    path: &RoutePath,
) -> Result<Vec<Rc<RouteSpace>>, SelectionError> {
    let Some((&material_index, parent_indices)) = path.0.split_last() else {
        return Ok(selector.routes.clone());
    };
    let parent_path = RoutePath(parent_indices.to_vec());
    let parent = selection_at(current, &parent_path)
        .ok_or_else(|| SelectionError::InvalidRoutePath(path.clone()))?;
    let fusion = match parent
        .space
        .choice(parent.choice_index)
        .ok_or(SelectionError::InvalidChoice(parent.choice_index))?
    {
        RouteChoice::Fusion(fusion) => fusion,
        RouteChoice::Direct(_) => return Err(SelectionError::InvalidMaterialIndex(material_index)),
    };
    let material = fusion
        .materials
        .get(material_index)
        .ok_or(SelectionError::InvalidMaterialIndex(material_index))?;
    Ok(material.routes.iter().cloned().collect())
}

fn selection_at<'a>(selection: &'a RouteSelection, path: &RoutePath) -> Option<&'a RouteSelection> {
    let mut current = selection;
    for &index in &path.0 {
        current = current.materials.get(index)?;
    }
    Some(current)
}

fn visible_choices(spaces: Vec<Rc<RouteSpace>>) -> Vec<VisibleChoice> {
    let mut seen = HashSet::new();
    let mut visible = Vec::new();
    for space in spaces {
        for (choice_index, choice) in space.choices.iter().enumerate() {
            let key = choice_key(space.fusion_depth, choice);
            if seen.insert(key.clone()) {
                visible.push(VisibleChoice {
                    key,
                    space: Rc::clone(&space),
                    choice_index,
                });
            }
        }
    }
    visible
}

fn selected_key(selection: &RouteSelection) -> Option<VisibleChoiceKey> {
    selection
        .space
        .choice(selection.choice_index)
        .map(|choice| choice_key(selection.space.fusion_depth, choice))
}

fn choice_key(route_depth: u32, choice: &RouteChoice) -> VisibleChoiceKey {
    match choice {
        RouteChoice::Direct(_) => VisibleChoiceKey::Direct,
        RouteChoice::Fusion(fusion) => {
            let mut materials = fusion.recipe.materials.clone();
            materials.sort_unstable();
            VisibleChoiceKey::Fusion {
                route_depth,
                materials,
            }
        }
    }
}

fn actual_depth(selection: &RouteSelection) -> u32 {
    if selection.materials.is_empty() {
        0
    } else {
        1 + selection
            .materials
            .iter()
            .map(actual_depth)
            .max()
            .unwrap_or(0)
    }
}

fn route_path(path: &[u8]) -> RoutePath {
    RoutePath(path.iter().copied().map(usize::from).collect())
}

fn failure(request_id: u64, code: WorkerFailureCode) -> WorkerResponse {
    WorkerResponse::Failure {
        request_id: Some(request_id),
        failure: failure_dto(code),
    }
}

fn search_failure(request_id: u64, error: SearchError) -> WorkerResponse {
    let failure = match error {
        SearchError::UnknownDemon(id) => failure_with_id(WorkerFailureCode::UnknownDemon, id.0),
        SearchError::UnknownSkill(id) => failure_with_id(WorkerFailureCode::UnknownSkill, id.0),
        SearchError::UnsupportedSkill(id) => {
            failure_with_id(WorkerFailureCode::UnsupportedSkill, id.0)
        }
        SearchError::TooManySkills { selected, maximum } => WorkerFailureDto {
            selected: Some(selected),
            maximum: Some(maximum),
            ..failure_dto(WorkerFailureCode::TooManySkills)
        },
        SearchError::UnavailableTarget(id) => {
            failure_with_id(WorkerFailureCode::UnavailableTarget, id.0)
        }
        SearchError::SafetyLimitExceeded(SearchSafetyLimit::ExpandedStates { maximum }) => {
            WorkerFailureDto {
                maximum: Some(maximum as usize),
                ..failure_dto(WorkerFailureCode::ExpandedStatesLimit)
            }
        }
        SearchError::SafetyLimitExceeded(SearchSafetyLimit::SkillAssignments { maximum }) => {
            WorkerFailureDto {
                maximum: Some(maximum as usize),
                ..failure_dto(WorkerFailureCode::SkillAssignmentsLimit)
            }
        }
    };
    WorkerResponse::Failure {
        request_id: Some(request_id),
        failure,
    }
}

fn selection_failure(request_id: u64, error: SelectionError) -> WorkerResponse {
    let code = match error {
        SelectionError::InvalidRoutePath(_) | SelectionError::InvalidMaterialIndex(_) => {
            WorkerFailureCode::InvalidPath
        }
        SelectionError::InvalidChoice(_)
        | SelectionError::InvalidFusionDepth(_)
        | SelectionError::IncompatibleMaterial(_) => WorkerFailureCode::InvalidOption,
        SelectionError::NoRoute => WorkerFailureCode::NoRoute,
    };
    failure(request_id, code)
}

fn failure_dto(code: WorkerFailureCode) -> WorkerFailureDto {
    WorkerFailureDto {
        code,
        related_id: None,
        selected: None,
        maximum: None,
    }
}

fn failure_with_id(code: WorkerFailureCode, related_id: u32) -> WorkerFailureDto {
    WorkerFailureDto {
        related_id: Some(related_id),
        ..failure_dto(code)
    }
}

pub fn invalid_message_response() -> WorkerResponse {
    WorkerResponse::Failure {
        request_id: None,
        failure: failure_dto(WorkerFailureCode::InvalidMessage),
    }
}

#[cfg(target_arch = "wasm32")]
pub fn run_worker() {
    use wasm_bindgen::{JsCast, closure::Closure};
    use web_sys::{DedicatedWorkerGlobalScope, MessageEvent};

    console_error_panic_hook::set_once();
    let scope: DedicatedWorkerGlobalScope = js_sys::global().unchecked_into();
    let mut service = WorkerService::new();
    let ready = service.ready();
    let response_scope = scope.clone();
    let on_message = Closure::<dyn FnMut(MessageEvent)>::new(move |event: MessageEvent| {
        let response = serde_wasm_bindgen::from_value::<WorkerRequest>(event.data())
            .map(|request| service.handle(request))
            .unwrap_or_else(|_| invalid_message_response());
        if let Ok(value) = serde_wasm_bindgen::to_value(&response) {
            let _ = response_scope.post_message(&value);
        }
    });
    scope.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
    on_message.forget();
    if let Ok(value) = serde_wasm_bindgen::to_value(&ready) {
        let _ = scope.post_message(&value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use smt5_fusion_core::{
        dataset::{demon_ids, skill_ids},
        model::skill::SkillId,
    };

    fn search_request(target: DemonId, skills: &[SkillId], depth: u32) -> WorkerRequest {
        WorkerRequest::Search {
            request_id: 1,
            input: SearchInputDto {
                target,
                required_skills: skills.to_vec(),
                max_fusion_depth: depth,
                dlc: crate::protocol::DlcSettingsDto::default(),
            },
        }
    }

    #[test]
    fn catalog_contains_formal_data_and_ordered_initial_skills() {
        let service = WorkerService::new();
        let WorkerResponse::Ready { catalog } = service.ready() else {
            panic!("expected ready response");
        };
        assert_eq!(catalog.demons.len(), 275);
        assert_eq!(catalog.skills.len(), 763);
        let pixie = &catalog.demons[demon_ids::PIXIE.0 as usize];
        assert_eq!(pixie.id, demon_ids::PIXIE);
        assert!(!pixie.initial_skills.is_empty());
        assert!(
            pixie
                .initial_skills
                .iter()
                .all(|id| catalog.skills[id.0 as usize].supported)
        );
    }

    #[test]
    fn search_returns_exact_count_and_concrete_tree() {
        let mut service = WorkerService::new();
        let response = service.handle(search_request(demon_ids::PIXIE, &[], 0));
        let WorkerResponse::SearchCompleted { result, .. } = response else {
            panic!("expected search result");
        };
        assert_eq!(result.route_count, "1");
        assert_eq!(result.actual_fusion_depth, 0);
        let tree = result.tree.expect("Pixie must have direct route");
        assert_eq!(tree.demon, demon_ids::PIXIE);
        assert!(matches!(tree.acquisition, AcquisitionDto::Direct { .. }));
    }

    #[test]
    fn upgrade_details_include_only_requested_skills() {
        let mut service = WorkerService::new();
        let response = service.handle(search_request(demon_ids::PIXIE, &[skill_ids::RAKUKAJA], 0));
        let WorkerResponse::SearchCompleted { result, .. } = response else {
            panic!("expected upgraded direct route");
        };
        let tree = result.tree.expect("Pixie learns Rakukaja by leveling");
        assert_eq!(tree.base_level, 2);
        assert_eq!(tree.final_level, 4);
        assert_eq!(
            tree.upgrade_skills,
            vec![UpgradeSkillDto {
                skill: skill_ids::RAKUKAJA,
                level: 4,
            }]
        );
    }

    #[test]
    fn shiva_riberama_count_respects_the_requested_maximum_depth() {
        let expected = [
            "0",
            "0",
            "498",
            "1009790648361",
            "67660618831471573302955414412367",
        ];
        for (depth, expected_count) in expected.into_iter().enumerate() {
            let mut service = WorkerService::new();
            let response = service.handle(search_request(
                demon_ids::SHIVA,
                &[skill_ids::RIBERAMA],
                depth as u32,
            ));
            let WorkerResponse::SearchCompleted { result, .. } = response else {
                panic!("expected Shiva search result at depth {depth}");
            };
            assert_eq!(result.input.max_fusion_depth, depth as u32);
            assert_eq!(result.route_count, expected_count);
        }
    }

    #[test]
    fn depth_four_big_count_crosses_the_protocol_without_precision_loss() {
        let mut service = WorkerService::new();
        let mut request = search_request(demon_ids::ALICE, &[skill_ids::AGI], 4);
        let WorkerRequest::Search { input, .. } = &mut request else {
            unreachable!();
        };
        input.dlc = crate::protocol::DlcSettingsDto {
            konohana_sakuya: true,
            dagda: true,
        };
        let response = service.handle(request);
        let WorkerResponse::SearchCompleted { result, .. } = response else {
            panic!("expected depth-four search result");
        };
        assert_eq!(
            result.route_count,
            "1666007940650373178057316178875587628948682855716928"
        );
        assert!(result.tree.is_some());
        assert!(result.actual_fusion_depth <= 4);
    }

    #[test]
    fn no_route_is_a_successful_empty_search_result() {
        let mut service = WorkerService::new();
        let response = service.handle(search_request(demon_ids::PIXIE, &[skill_ids::AGI], 0));
        let WorkerResponse::SearchCompleted { result, .. } = response else {
            panic!("expected empty search result");
        };
        assert_eq!(result.route_count, "0");
        assert!(result.tree.is_none());
    }

    #[test]
    fn shallow_default_keeps_deeper_route_spaces_selectable() {
        let mut service = WorkerService::new();
        let response = service.handle(search_request(demon_ids::SHIVA, &[skill_ids::RIBERAMA], 4));
        let WorkerResponse::SearchCompleted { result, .. } = response else {
            panic!("expected Shiva search result");
        };
        assert_eq!(result.actual_fusion_depth, 2);

        let options_response = service.handle(WorkerRequest::GetNodeOptions {
            request_id: 2,
            session_id: result.session_id,
            selection_revision: result.selection_revision,
            path: Vec::new(),
        });
        let WorkerResponse::NodeOptions { options, .. } = options_response else {
            panic!("expected Shiva root options");
        };
        let depths = options
            .options
            .iter()
            .filter_map(|option| match &option.acquisition {
                OptionAcquisitionDto::Fusion { route_depth, .. } => Some(*route_depth),
                OptionAcquisitionDto::Direct { .. } => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(depths, vec![2, 3, 4]);

        let depth_four = options
            .options
            .iter()
            .find(|option| {
                matches!(
                    &option.acquisition,
                    OptionAcquisitionDto::Fusion { route_depth: 4, .. }
                )
            })
            .expect("depth-four representative must remain visible");
        let changed = service.handle(WorkerRequest::SelectNodeOption {
            request_id: 3,
            session_id: result.session_id,
            selection_revision: result.selection_revision,
            path: Vec::new(),
            option_id: depth_four.option_id,
        });
        let WorkerResponse::SelectionChanged { snapshot, .. } = changed else {
            panic!("expected depth-four selection");
        };
        assert_eq!(snapshot.actual_fusion_depth, 4);

        let child_options = service.handle(WorkerRequest::GetNodeOptions {
            request_id: 4,
            session_id: snapshot.session_id,
            selection_revision: snapshot.selection_revision,
            path: vec![0],
        });
        assert!(matches!(
            child_options,
            WorkerResponse::NodeOptions {
                options: NodeOptionsDto { options, .. },
                ..
            } if options.iter().any(|option| matches!(
                &option.acquisition,
                OptionAcquisitionDto::Fusion { route_depth: 3, .. }
            ))
        ));
    }

    #[test]
    fn visible_options_are_unique_and_selection_updates_revision() {
        let mut service = WorkerService::new();
        let response = service.handle(search_request(demon_ids::PIXIE, &[skill_ids::AGI], 1));
        let WorkerResponse::SearchCompleted { result, .. } = response else {
            panic!("expected search result");
        };
        let options_response = service.handle(WorkerRequest::GetNodeOptions {
            request_id: 2,
            session_id: result.session_id,
            selection_revision: result.selection_revision,
            path: Vec::new(),
        });
        let WorkerResponse::NodeOptions { options, .. } = options_response else {
            panic!("expected options");
        };
        assert!(options.options.len() > 1);
        let mut keys = HashSet::new();
        for option in &options.options {
            let key = match &option.acquisition {
                OptionAcquisitionDto::Direct { .. } => (None, Vec::new()),
                OptionAcquisitionDto::Fusion {
                    route_depth,
                    materials,
                    ..
                } => {
                    let mut materials = materials.iter().map(|item| item.demon).collect::<Vec<_>>();
                    materials.sort_unstable();
                    (Some(*route_depth), materials)
                }
            };
            assert!(keys.insert(key));
        }
        let replacement = options
            .options
            .iter()
            .find(|option| !option.selected)
            .expect("another visible option must exist");
        let changed = service.handle(WorkerRequest::SelectNodeOption {
            request_id: 3,
            session_id: result.session_id,
            selection_revision: result.selection_revision,
            path: Vec::new(),
            option_id: replacement.option_id,
        });
        let WorkerResponse::SelectionChanged { snapshot, .. } = changed else {
            panic!("expected changed selection");
        };
        assert_eq!(snapshot.selection_revision, 1);
        assert_eq!(snapshot.tree.demon, demon_ids::PIXIE);
    }

    #[test]
    fn direct_fusion_child_selection_and_reset_form_a_closed_session_flow() {
        let mut service = WorkerService::new();
        let response = service.handle(search_request(demon_ids::PIXIE, &[], 1));
        let WorkerResponse::SearchCompleted { result, .. } = response else {
            panic!("expected search result");
        };
        let options_response = service.handle(WorkerRequest::GetNodeOptions {
            request_id: 2,
            session_id: result.session_id,
            selection_revision: result.selection_revision,
            path: Vec::new(),
        });
        let WorkerResponse::NodeOptions { options, .. } = options_response else {
            panic!("expected options");
        };
        assert!(matches!(
            result.tree.as_ref().map(|tree| &tree.acquisition),
            Some(AcquisitionDto::Direct { .. })
        ));
        assert!(matches!(
            options.options.first(),
            Some(VisibleOptionDto {
                option_id: 0,
                selected: true,
                acquisition: OptionAcquisitionDto::Direct { .. },
            })
        ));
        let fusion = options
            .options
            .iter()
            .find(|option| matches!(&option.acquisition, OptionAcquisitionDto::Fusion { .. }))
            .expect("depth-one Pixie must have a fusion option");
        let changed = service.handle(WorkerRequest::SelectNodeOption {
            request_id: 3,
            session_id: result.session_id,
            selection_revision: result.selection_revision,
            path: Vec::new(),
            option_id: fusion.option_id,
        });
        let WorkerResponse::SelectionChanged { snapshot, .. } = changed else {
            panic!("expected fusion selection");
        };
        assert!(matches!(
            snapshot.tree.acquisition,
            AcquisitionDto::Fusion { .. }
        ));
        assert_eq!(snapshot.actual_fusion_depth, 1);
        assert!(!snapshot.tree.children.is_empty());

        let child_options = service.handle(WorkerRequest::GetNodeOptions {
            request_id: 4,
            session_id: snapshot.session_id,
            selection_revision: snapshot.selection_revision,
            path: vec![0],
        });
        assert!(matches!(
            child_options,
            WorkerResponse::NodeOptions {
                options: NodeOptionsDto { options, .. },
                ..
            } if !options.is_empty()
        ));

        let reset = service.handle(WorkerRequest::ResetToDefault {
            request_id: 5,
            session_id: snapshot.session_id,
            selection_revision: snapshot.selection_revision,
        });
        let WorkerResponse::SelectionChanged { snapshot, .. } = reset else {
            panic!("expected reset selection");
        };
        assert_eq!(snapshot.selection_revision, 2);
        assert_eq!(snapshot.actual_fusion_depth, 0);
        assert!(matches!(
            snapshot.tree.acquisition,
            AcquisitionDto::Direct { .. }
        ));
    }

    #[test]
    fn unavailable_dlc_target_returns_a_domain_failure() {
        let mut service = WorkerService::new();
        let response = service.handle(search_request(demon_ids::KONOHANA_SAKUYA, &[], 0));
        assert!(matches!(
            response,
            WorkerResponse::Failure {
                failure: WorkerFailureDto {
                    code: WorkerFailureCode::UnavailableTarget,
                    related_id: Some(id),
                    ..
                },
                ..
            } if id == demon_ids::KONOHANA_SAKUYA.0
        ));
    }

    #[test]
    fn stale_selection_is_rejected() {
        let mut service = WorkerService::new();
        let response = service.handle(search_request(demon_ids::PIXIE, &[], 0));
        let WorkerResponse::SearchCompleted { result, .. } = response else {
            panic!("expected search result");
        };
        let response = service.handle(WorkerRequest::GetNodeOptions {
            request_id: 2,
            session_id: result.session_id,
            selection_revision: result.selection_revision + 1,
            path: Vec::new(),
        });
        assert!(matches!(
            response,
            WorkerResponse::Failure {
                failure: WorkerFailureDto {
                    code: WorkerFailureCode::StaleSelection,
                    ..
                },
                ..
            }
        ));
    }
}
