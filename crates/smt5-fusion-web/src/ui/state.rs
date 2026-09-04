use std::{
    collections::BTreeSet,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
};

use leptos::prelude::*;
use send_wrapper::SendWrapper;
use serde::{Deserialize, Serialize};

use crate::{
    i18n::{demon_name, text},
    protocol::{
        CatalogDto, DemonContent, DemonId, DlcSettingsDto, MAX_FUSION_DEPTH, NodeOptionsDto,
        RouteTreeNodeDto, SearchInputDto, SearchResultDto, SelectionSnapshotDto, SkillCategory,
        SkillId, WorkerFailureCode, WorkerFailureDto, WorkerRequest, WorkerResponse,
    },
};

use super::{
    selectors::{default_collapsed, demon, demon_available, skill, skill_eligible_for_target},
    worker_client::WorkerClient,
};

#[cfg(target_arch = "wasm32")]
const STORAGE_KEY: &str = "smt5-fusion-web:search-form:v1";
const DEFAULT_FUSION_DEPTH: u32 = 2;
pub(super) const OPTION_BATCH_SIZE: usize = 50;
pub(super) const SKILL_CAPACITY: usize = 8;

#[derive(Clone, Copy)]
pub(super) struct AppState {
    pub(super) catalog: RwSignal<Option<Arc<CatalogDto>>>,
    pub(super) worker_ready: RwSignal<bool>,
    pub(super) searching: RwSignal<bool>,
    pub(super) selection_busy: RwSignal<bool>,
    pub(super) active_search: RwSignal<Option<u64>>,
    pub(super) active_options: RwSignal<Option<u64>>,
    pub(super) active_mutation: RwSignal<Option<u64>>,
    pub(super) result: RwSignal<Option<Arc<SearchResultDto>>>,
    pub(super) options: RwSignal<Option<Arc<NodeOptionsDto>>>,
    pub(super) options_loading: RwSignal<bool>,
    pub(super) panel_path: RwSignal<Option<Vec<u8>>>,
    pub(super) collapsed: RwSignal<BTreeSet<Vec<u8>>>,
    pub(super) error: RwSignal<Option<String>>,
    pub(super) dirty: RwSignal<bool>,
    pub(super) target: RwSignal<Option<DemonId>>,
    pub(super) required_skills: RwSignal<Vec<SkillId>>,
    pub(super) max_depth: RwSignal<u32>,
    pub(super) konohana_sakuya_dlc: RwSignal<bool>,
    pub(super) dagda_dlc: RwSignal<bool>,
    pub(super) target_query: RwSignal<String>,
    pub(super) target_picker_open: RwSignal<bool>,
    pub(super) target_active_index: RwSignal<usize>,
    pub(super) skill_picker_open: RwSignal<bool>,
    pub(super) skill_query: RwSignal<String>,
    pub(super) skill_category: RwSignal<Option<SkillCategory>>,
    pub(super) option_query: RwSignal<String>,
    pub(super) option_active_index: RwSignal<usize>,
    pub(super) option_limit: RwSignal<usize>,
}

impl AppState {
    pub(super) fn new(form: PersistedForm) -> Self {
        Self {
            catalog: RwSignal::new(None),
            worker_ready: RwSignal::new(false),
            searching: RwSignal::new(false),
            selection_busy: RwSignal::new(false),
            active_search: RwSignal::new(None),
            active_options: RwSignal::new(None),
            active_mutation: RwSignal::new(None),
            result: RwSignal::new(None),
            options: RwSignal::new(None),
            options_loading: RwSignal::new(false),
            panel_path: RwSignal::new(None),
            collapsed: RwSignal::new(BTreeSet::new()),
            error: RwSignal::new(None),
            dirty: RwSignal::new(false),
            target: RwSignal::new(form.target),
            required_skills: RwSignal::new(form.required_skills),
            max_depth: RwSignal::new(form.max_depth),
            konohana_sakuya_dlc: RwSignal::new(form.konohana_sakuya_dlc),
            dagda_dlc: RwSignal::new(form.dagda_dlc),
            target_query: RwSignal::new(String::new()),
            target_picker_open: RwSignal::new(false),
            target_active_index: RwSignal::new(0),
            skill_picker_open: RwSignal::new(false),
            skill_query: RwSignal::new(String::new()),
            skill_category: RwSignal::new(None),
            option_query: RwSignal::new(String::new()),
            option_active_index: RwSignal::new(0),
            option_limit: RwSignal::new(OPTION_BATCH_SIZE),
        }
    }

    pub(super) fn handle_response(self, response: WorkerResponse) {
        match response {
            WorkerResponse::Ready { catalog } => {
                self.sanitize_form(&catalog);
                self.target_query.set(
                    self.target
                        .get_untracked()
                        .map(demon_name)
                        .unwrap_or_default()
                        .to_owned(),
                );
                self.catalog.set(Some(Arc::new(catalog)));
                self.worker_ready.set(true);
                self.error.set(None);
                save_form(PersistedForm::from_state(self));
            }
            WorkerResponse::SearchCompleted { request_id, result } => {
                if self.active_search.get_untracked() != Some(request_id) {
                    return;
                }
                let dirty = self.current_input().as_ref() != Some(&result.input);
                let collapsed = default_collapsed(result.tree.as_ref());
                batch(move || {
                    self.active_search.set(None);
                    self.searching.set(false);
                    self.dirty.set(dirty);
                    self.collapsed.set(collapsed);
                    self.result.set(Some(Arc::new(result)));
                    self.close_options();
                });
            }
            WorkerResponse::NodeOptions {
                request_id,
                options,
            } => {
                if self.active_options.get_untracked() != Some(request_id) {
                    return;
                }
                self.active_options.set(None);
                self.options_loading.set(false);
                if self.current_session_matches(options.session_id, options.selection_revision) {
                    let selected_index = options
                        .options
                        .iter()
                        .position(|option| option.selected)
                        .unwrap_or(0);
                    self.option_active_index.set(selected_index);
                    self.option_limit
                        .set((selected_index + 1).max(OPTION_BATCH_SIZE));
                    self.options.set(Some(Arc::new(options)));
                }
            }
            WorkerResponse::SelectionChanged {
                request_id,
                snapshot,
            } => {
                if self.active_mutation.get_untracked() != Some(request_id) {
                    return;
                }
                batch(move || {
                    self.active_mutation.set(None);
                    self.selection_busy.set(false);
                    self.apply_snapshot(snapshot);
                    self.close_options();
                });
            }
            WorkerResponse::Failure {
                request_id,
                failure,
            } => {
                let search_failed = request_id == self.active_search.get_untracked();
                let options_failed = request_id == self.active_options.get_untracked();
                let mutation_failed = request_id == self.active_mutation.get_untracked();
                if request_id.is_some() && !search_failed && !options_failed && !mutation_failed {
                    return;
                }
                let message = failure_message(&failure);
                batch(|| {
                    if search_failed {
                        self.active_search.set(None);
                        self.searching.set(false);
                    }
                    if options_failed {
                        self.close_options();
                    }
                    if mutation_failed {
                        self.active_mutation.set(None);
                        self.selection_busy.set(false);
                    }
                    self.error.set(Some(message));
                });
            }
        }
    }

    fn apply_snapshot(self, snapshot: SelectionSnapshotDto) {
        let Some(result) = self.result.get_untracked() else {
            return;
        };
        if result.session_id != snapshot.session_id {
            return;
        }
        let changed_path = self.panel_path.get_untracked();
        let mut updated = result.as_ref().clone();
        updated.selection_revision = snapshot.selection_revision;
        updated.actual_fusion_depth = snapshot.actual_fusion_depth;
        updated.tree = Some(snapshot.tree);
        self.update_collapsed(updated.tree.as_ref(), changed_path.as_deref());
        self.result.set(Some(Arc::new(updated)));
    }

    fn update_collapsed(self, tree: Option<&RouteTreeNodeDto>, changed_path: Option<&[u8]>) {
        let defaults = default_collapsed(tree);
        match changed_path {
            Some(changed) => self.collapsed.update(|collapsed| {
                collapsed.retain(|path| !path.starts_with(changed));
                collapsed.extend(
                    defaults
                        .into_iter()
                        .filter(|path| path.starts_with(changed)),
                );
            }),
            None => self.collapsed.set(defaults),
        }
    }

    fn current_session_matches(self, session_id: u64, revision: u64) -> bool {
        self.result.get_untracked().is_some_and(|result| {
            result.session_id == session_id && result.selection_revision == revision
        })
    }

    pub(super) fn close_options(self) {
        self.panel_path.set(None);
        self.options.set(None);
        self.options_loading.set(false);
        self.active_options.set(None);
    }

    pub(super) fn current_input(self) -> Option<SearchInputDto> {
        Some(SearchInputDto {
            target: self.target.get_untracked()?,
            required_skills: self.required_skills.get_untracked(),
            max_fusion_depth: self.max_depth.get_untracked(),
            dlc: self.dlc_settings(),
        })
    }

    pub(super) fn dlc_settings(self) -> DlcSettingsDto {
        DlcSettingsDto {
            konohana_sakuya: self.konohana_sakuya_dlc.get_untracked(),
            dagda: self.dagda_dlc.get_untracked(),
        }
    }

    fn sanitize_form(self, catalog: &CatalogDto) {
        self.max_depth
            .update(|depth| *depth = (*depth).min(MAX_FUSION_DEPTH));
        let Some(target_id) = self.target.get_untracked() else {
            self.required_skills.set(Vec::new());
            return;
        };
        let Some(target) = demon(catalog, target_id) else {
            self.target.set(None);
            self.required_skills.set(Vec::new());
            return;
        };
        if !demon_available(target, self.dlc_settings()) {
            self.target.set(None);
            self.required_skills.set(Vec::new());
            return;
        }
        let mut unique = BTreeSet::new();
        self.required_skills.update(|selected| {
            selected.retain(|skill_id| {
                unique.insert(*skill_id)
                    && skill(catalog, *skill_id)
                        .is_some_and(|candidate| skill_eligible_for_target(target, candidate))
            });
            selected.truncate(SKILL_CAPACITY);
        });
    }
}

#[derive(Clone)]
pub(super) struct Controller {
    pub(super) state: AppState,
    worker: Arc<Mutex<Option<SendWrapper<WorkerClient>>>>,
    next_request_id: Arc<AtomicU64>,
    on_response: Arc<dyn Fn(WorkerResponse) + Send + Sync>,
    on_error: Arc<dyn Fn(String) + Send + Sync>,
}

impl Controller {
    pub(super) fn new(state: AppState) -> Self {
        let response_state = state;
        let error_state = state;
        Self {
            state,
            worker: Arc::new(Mutex::new(None)),
            next_request_id: Arc::new(AtomicU64::new(1)),
            on_response: Arc::new(move |response| response_state.handle_response(response)),
            on_error: Arc::new(move |details| {
                let message = if details.is_empty() {
                    text::WORKER_FAILED.to_owned()
                } else {
                    text::worker_failed_with_details(&details)
                };
                batch(|| {
                    error_state.worker_ready.set(false);
                    error_state.searching.set(false);
                    error_state.selection_busy.set(false);
                    error_state.active_search.set(None);
                    error_state.active_mutation.set(None);
                    error_state.close_options();
                    error_state.error.set(Some(message));
                });
            }),
        }
    }

    pub(super) fn start_worker(&self) {
        self.state.worker_ready.set(false);
        self.state.searching.set(false);
        self.state.selection_busy.set(false);
        self.state.active_search.set(None);
        self.state.active_mutation.set(None);
        self.state.result.set(None);
        self.state.close_options();
        self.worker
            .lock()
            .expect("worker lock must be available")
            .take();
        match WorkerClient::new(Arc::clone(&self.on_response), Arc::clone(&self.on_error)) {
            Ok(worker) => {
                *self.worker.lock().expect("worker lock must be available") =
                    Some(SendWrapper::new(worker));
            }
            Err(error) => (self.on_error)(error),
        }
    }

    fn send(&self, request: WorkerRequest) -> bool {
        let result = self
            .worker
            .lock()
            .expect("worker lock must be available")
            .as_ref()
            .ok_or_else(|| text::WORKER_FAILED.to_owned())
            .and_then(|worker| worker.send(&request));
        if let Err(error) = result {
            (self.on_error)(error);
            return false;
        }
        true
    }

    fn next_request(&self) -> u64 {
        self.next_request_id.fetch_add(1, Ordering::Relaxed)
    }

    pub(super) fn search(&self) {
        if self.state.searching.get_untracked() {
            return;
        }
        let Some(input) = self.state.current_input() else {
            self.state
                .error
                .set(Some(text::SELECT_TARGET_ERROR.to_owned()));
            return;
        };
        if !self.state.worker_ready.get_untracked() {
            self.state.error.set(Some(text::WORKER_FAILED.to_owned()));
            return;
        }
        let request_id = self.next_request();
        self.state.error.set(None);
        self.state.active_search.set(Some(request_id));
        self.state.searching.set(true);
        self.state.close_options();
        if !self.send(WorkerRequest::Search { request_id, input }) {
            self.state.active_search.set(None);
            self.state.searching.set(false);
        }
    }

    pub(super) fn open_options(&self, path: Vec<u8>) {
        if self.state.searching.get_untracked() || !self.state.worker_ready.get_untracked() {
            return;
        }
        let Some(result) = self.state.result.get_untracked() else {
            return;
        };
        let request_id = self.next_request();
        self.state.panel_path.set(Some(path.clone()));
        self.state.options.set(None);
        self.state.options_loading.set(true);
        self.state.option_query.set(String::new());
        self.state.option_active_index.set(0);
        self.state.option_limit.set(OPTION_BATCH_SIZE);
        self.state.active_options.set(Some(request_id));
        if !self.send(WorkerRequest::GetNodeOptions {
            request_id,
            session_id: result.session_id,
            selection_revision: result.selection_revision,
            path,
        }) {
            self.state.active_options.set(None);
            self.state.options_loading.set(false);
        }
    }

    pub(super) fn select_option(&self, option_id: u32) {
        if !self.state.worker_ready.get_untracked() {
            return;
        }
        let (Some(result), Some(path)) = (
            self.state.result.get_untracked(),
            self.state.panel_path.get_untracked(),
        ) else {
            return;
        };
        let request_id = self.next_request();
        self.state.selection_busy.set(true);
        self.state.active_mutation.set(Some(request_id));
        if !self.send(WorkerRequest::SelectNodeOption {
            request_id,
            session_id: result.session_id,
            selection_revision: result.selection_revision,
            path,
            option_id,
        }) {
            self.state.active_mutation.set(None);
            self.state.selection_busy.set(false);
        }
    }

    pub(super) fn reset_default(&self) {
        if self.state.searching.get_untracked() || !self.state.worker_ready.get_untracked() {
            return;
        }
        let Some(result) = self.state.result.get_untracked() else {
            return;
        };
        let request_id = self.next_request();
        self.state.panel_path.set(None);
        self.state.selection_busy.set(true);
        self.state.active_mutation.set(Some(request_id));
        if !self.send(WorkerRequest::ResetToDefault {
            request_id,
            session_id: result.session_id,
            selection_revision: result.selection_revision,
        }) {
            self.state.active_mutation.set(None);
            self.state.selection_busy.set(false);
        }
    }

    pub(super) fn update_target_query(&self, query: String) {
        self.state.target_query.set(query.clone());
        self.state.target_picker_open.set(true);
        self.state.target_active_index.set(0);
        if self
            .state
            .target
            .get_untracked()
            .is_some_and(|target| demon_name(target) != query.trim())
        {
            self.state.target.set(None);
            self.state.required_skills.set(Vec::new());
            self.form_changed();
        }
    }

    pub(super) fn select_target(&self, target: Option<DemonId>) {
        self.state.target.set(target);
        self.state
            .target_query
            .set(target.map(demon_name).unwrap_or_default().to_owned());
        self.state.target_picker_open.set(false);
        self.state.target_active_index.set(0);
        let initial = target
            .and_then(|id| {
                self.state.catalog.get_untracked().and_then(|catalog| {
                    demon(&catalog, id).map(|demon| {
                        demon
                            .initial_skills
                            .iter()
                            .copied()
                            .filter(|skill_id| {
                                skill(&catalog, *skill_id).is_some_and(|skill| skill.supported)
                            })
                            .take(SKILL_CAPACITY)
                            .collect()
                    })
                })
            })
            .unwrap_or_default();
        self.state.required_skills.set(initial);
        self.form_changed();
    }

    pub(super) fn set_dlc(&self, content: DemonContent, enabled: bool) {
        match content {
            DemonContent::KonohanaSakuyaDlc => self.state.konohana_sakuya_dlc.set(enabled),
            DemonContent::DagdaDlc => self.state.dagda_dlc.set(enabled),
            DemonContent::Base => return,
        }
        if let (Some(catalog), Some(target_id)) = (
            self.state.catalog.get_untracked(),
            self.state.target.get_untracked(),
        ) && demon(&catalog, target_id)
            .is_some_and(|target| !demon_available(target, self.state.dlc_settings()))
        {
            self.state.target.set(None);
            self.state.target_query.set(String::new());
            self.state.target_picker_open.set(false);
            self.state.required_skills.set(Vec::new());
        }
        self.form_changed();
    }

    pub(super) fn set_depth(&self, depth: u32) {
        self.state.max_depth.set(depth.min(MAX_FUSION_DEPTH));
        self.form_changed();
    }

    pub(super) fn add_skill(&self, skill_id: SkillId) {
        if self.state.required_skills.get_untracked().len() >= SKILL_CAPACITY {
            return;
        }
        let eligible = match (
            self.state.catalog.get_untracked(),
            self.state.target.get_untracked(),
        ) {
            (Some(catalog), Some(target_id)) => {
                let Some(target) = demon(&catalog, target_id) else {
                    return;
                };
                skill(&catalog, skill_id)
                    .is_some_and(|candidate| skill_eligible_for_target(target, candidate))
            }
            _ => false,
        };
        if !eligible
            || self
                .state
                .required_skills
                .get_untracked()
                .contains(&skill_id)
        {
            return;
        }
        self.state
            .required_skills
            .update(|skills| skills.push(skill_id));
        self.state.skill_picker_open.set(false);
        self.form_changed();
    }

    pub(super) fn remove_skill(&self, index: usize) {
        self.state.required_skills.update(|skills| {
            if index < skills.len() {
                skills.remove(index);
            }
        });
        self.form_changed();
    }

    pub(super) fn clear_form(&self) {
        batch(|| {
            self.state.active_search.set(None);
            self.state.active_mutation.set(None);
            self.state.searching.set(false);
            self.state.selection_busy.set(false);
            self.state.result.set(None);
            self.state.collapsed.set(BTreeSet::new());
            self.state.dirty.set(false);
            self.state.target.set(None);
            self.state.required_skills.set(Vec::new());
            self.state.max_depth.set(DEFAULT_FUSION_DEPTH);
            self.state.konohana_sakuya_dlc.set(false);
            self.state.dagda_dlc.set(false);
            self.state.target_query.set(String::new());
            self.state.target_picker_open.set(false);
            self.state.target_active_index.set(0);
            self.state.close_options();
        });
        save_form(PersistedForm::from_state(self.state));
    }

    fn form_changed(&self) {
        if self.state.result.get_untracked().is_some() {
            self.state.dirty.set(true);
        }
        save_form(PersistedForm::from_state(self.state));
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct PersistedForm {
    pub(super) version: u8,
    pub(super) target: Option<DemonId>,
    pub(super) required_skills: Vec<SkillId>,
    pub(super) max_depth: u32,
    pub(super) konohana_sakuya_dlc: bool,
    pub(super) dagda_dlc: bool,
}

impl Default for PersistedForm {
    fn default() -> Self {
        Self {
            version: 1,
            target: None,
            required_skills: Vec::new(),
            max_depth: DEFAULT_FUSION_DEPTH,
            konohana_sakuya_dlc: false,
            dagda_dlc: false,
        }
    }
}

impl PersistedForm {
    fn from_state(state: AppState) -> Self {
        Self {
            version: 1,
            target: state.target.get_untracked(),
            required_skills: state.required_skills.get_untracked(),
            max_depth: state.max_depth.get_untracked(),
            konohana_sakuya_dlc: state.konohana_sakuya_dlc.get_untracked(),
            dagda_dlc: state.dagda_dlc.get_untracked(),
        }
    }
}

fn failure_message(failure: &WorkerFailureDto) -> String {
    match failure.code {
        WorkerFailureCode::UnknownDemon => text::ERROR_UNKNOWN_DEMON.to_owned(),
        WorkerFailureCode::UnknownSkill => text::ERROR_UNKNOWN_SKILL.to_owned(),
        WorkerFailureCode::UnsupportedSkill => text::ERROR_UNSUPPORTED_SKILL.to_owned(),
        WorkerFailureCode::TooManySkills => text::too_many_skills(
            failure.selected.unwrap_or_default(),
            failure.maximum.unwrap_or(SKILL_CAPACITY),
        ),
        WorkerFailureCode::UnavailableTarget => text::ERROR_UNAVAILABLE_TARGET.to_owned(),
        WorkerFailureCode::ExpandedStatesLimit | WorkerFailureCode::SkillAssignmentsLimit => {
            text::ERROR_SAFETY_LIMIT.to_owned()
        }
        WorkerFailureCode::InvalidSession | WorkerFailureCode::StaleSelection => {
            text::ERROR_STALE_SELECTION.to_owned()
        }
        WorkerFailureCode::InvalidPath | WorkerFailureCode::InvalidOption => {
            text::ERROR_INVALID_OPTION.to_owned()
        }
        WorkerFailureCode::NoRoute => text::NO_ROUTE.to_owned(),
        WorkerFailureCode::InvalidMessage | WorkerFailureCode::Internal => {
            text::ERROR_INTERNAL.to_owned()
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub(super) fn load_form() -> PersistedForm {
    web_sys::window()
        .and_then(|window| window.local_storage().ok().flatten())
        .and_then(|storage| storage.get_item(STORAGE_KEY).ok().flatten())
        .and_then(|value| serde_json::from_str::<PersistedForm>(&value).ok())
        .filter(|form| form.version == 1 && form.max_depth <= MAX_FUSION_DEPTH)
        .unwrap_or_default()
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn load_form() -> PersistedForm {
    PersistedForm::default()
}

#[cfg(target_arch = "wasm32")]
fn save_form(form: PersistedForm) {
    let Some(storage) = web_sys::window().and_then(|window| window.local_storage().ok().flatten())
    else {
        return;
    };
    if let Ok(value) = serde_json::to_string(&form) {
        let _ = storage.set_item(STORAGE_KEY, &value);
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn save_form(_form: PersistedForm) {}
