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
    i18n::{I18n, Locale, Message},
    protocol::{
        CatalogDto, DemonContent, DemonId, DlcSettingsDto, MAX_FUSION_DEPTH, NodeOptionsDto,
        RouteTreeNodeDto, SearchInputDto, SearchResultDto, SelectionSnapshotDto, SkillCategory,
        SkillId, WorkerFailureCode, WorkerFailureDto, WorkerRequest, WorkerResponse,
    },
};

use super::{
    search::prepare_search_indexes,
    selectors::{
        default_collapsed, demon, demon_available, filtered_options, skill,
        skill_eligible_for_target,
    },
    worker_client::WorkerClient,
};

#[cfg(target_arch = "wasm32")]
const STORAGE_KEY: &str = "smt5-fusion-web:search-form:v1";
const DEFAULT_FUSION_DEPTH: u32 = 2;
#[cfg(target_arch = "wasm32")]
const LOADING_INDICATOR_DELAY_MS: i32 = 100;
pub(super) const SKILL_CAPACITY: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum WorkerStatus {
    Loading,
    Ready,
    Failed,
}

#[derive(Clone, Copy)]
pub(super) struct AppState {
    pub(super) catalog: RwSignal<Option<Arc<CatalogDto>>>,
    pub(super) worker_status: RwSignal<WorkerStatus>,
    pub(super) worker_generation: RwSignal<u64>,
    pub(super) session_available: RwSignal<bool>,
    pub(super) searching: RwSignal<bool>,
    pub(super) search_indicator_visible: RwSignal<bool>,
    pub(super) selection_busy: RwSignal<bool>,
    pub(super) active_search: RwSignal<Option<u64>>,
    pub(super) active_options: RwSignal<Option<u64>>,
    pub(super) active_mutation: RwSignal<Option<u64>>,
    pub(super) mutation_path: RwSignal<Option<Vec<u8>>>,
    pub(super) result: RwSignal<Option<Arc<SearchResultDto>>>,
    pub(super) options: RwSignal<Option<Arc<NodeOptionsDto>>>,
    pub(super) options_indicator_visible: RwSignal<bool>,
    pub(super) panel_path: RwSignal<Option<Vec<u8>>>,
    pub(super) collapsed: RwSignal<BTreeSet<Vec<u8>>>,
    pub(super) error: RwSignal<Option<AppError>>,
    pub(super) i18n: I18n,
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
}

impl AppState {
    pub(super) fn new(form: PersistedForm, i18n: I18n) -> Self {
        Self {
            catalog: RwSignal::new(None),
            worker_status: RwSignal::new(WorkerStatus::Loading),
            worker_generation: RwSignal::new(0),
            session_available: RwSignal::new(false),
            searching: RwSignal::new(false),
            search_indicator_visible: RwSignal::new(false),
            selection_busy: RwSignal::new(false),
            active_search: RwSignal::new(None),
            active_options: RwSignal::new(None),
            active_mutation: RwSignal::new(None),
            mutation_path: RwSignal::new(None),
            result: RwSignal::new(None),
            options: RwSignal::new(None),
            options_indicator_visible: RwSignal::new(false),
            panel_path: RwSignal::new(None),
            collapsed: RwSignal::new(BTreeSet::new()),
            error: RwSignal::new(None),
            i18n,
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
        }
    }

    pub(super) fn worker_ready(self) -> bool {
        self.worker_status.get() == WorkerStatus::Ready
    }

    pub(super) fn can_search(self) -> bool {
        self.worker_ready()
            && !self.searching.get()
            && !self.selection_busy.get()
            && self.target.get().is_some()
    }

    pub(super) fn can_edit_route(self) -> bool {
        self.worker_ready()
            && self.session_available.get()
            && !self.searching.get()
            && !self.selection_busy.get()
            && self
                .result
                .with(|result| result.as_ref().is_some_and(|result| result.tree.is_some()))
    }

    pub(super) fn can_edit_skills(self) -> bool {
        self.worker_ready() && self.catalog.get().is_some() && self.target.get().is_some()
    }

    pub(super) fn result_is_stale(self) -> bool {
        self.result.with(|result| {
            result.as_ref().is_some_and(|result| {
                self.target.get() != Some(result.input.target)
                    || self.max_depth.get() != result.input.max_fusion_depth
                    || self.konohana_sakuya_dlc.get() != result.input.dlc.konohana_sakuya
                    || self.dagda_dlc.get() != result.input.dlc.dagda
                    || self.required_skills.with(|skills| {
                        skills.len() != result.input.required_skills.len()
                            || skills
                                .iter()
                                .any(|id| !result.input.required_skills.contains(id))
                    })
            })
        })
    }

    fn invalidate_requests(self) {
        self.active_search.set(None);
        self.searching.set(false);
        self.search_indicator_visible.set(false);
        self.active_mutation.set(None);
        self.mutation_path.set(None);
        self.selection_busy.set(false);
        self.session_available.set(false);
        self.close_options();
    }

    pub(super) fn close_skill_picker(self) {
        self.skill_picker_open.set(false);
        self.skill_query.set(String::new());
        self.skill_category.set(None);
    }

    fn close_pickers(self) {
        self.target_picker_open.set(false);
        self.target_active_index.set(0);
        self.close_skill_picker();
        self.close_options();
    }

    fn begin_worker_start(self) -> u64 {
        let generation = self.worker_generation.get_untracked().wrapping_add(1);
        batch(|| {
            self.worker_generation.set(generation);
            self.worker_status.set(WorkerStatus::Loading);
            self.invalidate_requests();
            self.close_pickers();
            self.result.set(None);
            self.collapsed.set(BTreeSet::new());
            self.error.set(None);
        });
        generation
    }

    pub(super) fn handle_worker_response(self, generation: u64, response: WorkerResponse) {
        if self.worker_generation.try_get_untracked() != Some(generation) {
            return;
        }
        let expected_status = if matches!(&response, WorkerResponse::Ready { .. }) {
            WorkerStatus::Loading
        } else {
            WorkerStatus::Ready
        };
        if self.worker_status.try_get_untracked() == Some(expected_status) {
            self.handle_response(response);
        }
    }

    pub(super) fn handle_worker_error(self, generation: u64, details: String) {
        if self.worker_generation.try_get_untracked() == Some(generation)
            && self.worker_status.try_get_untracked() != Some(WorkerStatus::Failed)
        {
            self.worker_failed(AppError::WorkerFailed(details));
        }
    }

    fn worker_failed(self, error: AppError) {
        batch(|| {
            self.worker_status.set(WorkerStatus::Failed);
            self.invalidate_requests();
            self.close_pickers();
            self.error.set(Some(error));
        });
    }

    pub(super) fn begin_search(self, request_id: u64) {
        batch(|| {
            self.error.set(None);
            self.session_available.set(false);
            self.active_search.set(Some(request_id));
            self.searching.set(true);
            self.search_indicator_visible.set(false);
            self.close_options();
        });
    }

    pub(super) fn handle_response(self, response: WorkerResponse) {
        match response {
            WorkerResponse::Ready { catalog } => {
                prepare_search_indexes();
                self.sanitize_form(&catalog);
                self.target_query.set(
                    self.target
                        .get_untracked()
                        .map(|id| self.i18n.demon_name_untracked(id))
                        .unwrap_or_default(),
                );
                self.catalog.set(Some(Arc::new(catalog)));
                self.worker_status.set(WorkerStatus::Ready);
                self.search_indicator_visible.set(false);
                self.error.set(None);
                save_form(PersistedForm::from_state(self));
            }
            WorkerResponse::SearchCompleted { request_id, result } => {
                if self.active_search.get_untracked() != Some(request_id) {
                    return;
                }
                let collapsed = default_collapsed(result.tree.as_ref());
                batch(move || {
                    self.active_search.set(None);
                    self.searching.set(false);
                    self.search_indicator_visible.set(false);
                    self.session_available.set(true);
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
                batch(|| {
                    self.active_options.set(None);
                    self.options_indicator_visible.set(false);
                    if self.current_session_matches(options.session_id, options.selection_revision)
                        && self.panel_path.get_untracked().as_ref() == Some(&options.path)
                    {
                        self.options.set(Some(Arc::new(options)));
                        let selected_index = untrack(|| filtered_options(self))
                            .iter()
                            .position(|option| option.selected)
                            .unwrap_or(0);
                        self.option_active_index.set(selected_index);
                    } else {
                        self.close_options();
                    }
                });
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
                    let changed_path = self.mutation_path.get_untracked();
                    self.mutation_path.set(None);
                    self.apply_snapshot(snapshot, changed_path.as_deref());
                    self.close_options();
                });
            }
            WorkerResponse::Failure {
                request_id,
                failure,
            } => {
                if request_id.is_none() {
                    self.worker_failed(AppError::WorkerFailure(failure));
                    return;
                }
                let search_failed = request_id == self.active_search.get_untracked();
                let options_failed = request_id == self.active_options.get_untracked();
                let mutation_failed = request_id == self.active_mutation.get_untracked();
                if request_id.is_some() && !search_failed && !options_failed && !mutation_failed {
                    return;
                }
                batch(|| {
                    if search_failed {
                        self.active_search.set(None);
                        self.searching.set(false);
                        self.search_indicator_visible.set(false);
                        self.session_available.set(false);
                    }
                    if matches!(
                        failure.code,
                        WorkerFailureCode::InvalidSession | WorkerFailureCode::StaleSelection
                    ) {
                        self.session_available.set(false);
                        self.close_options();
                    }
                    if options_failed {
                        self.close_options();
                    }
                    if mutation_failed {
                        self.active_mutation.set(None);
                        self.mutation_path.set(None);
                        self.selection_busy.set(false);
                    }
                    self.error.set(Some(AppError::WorkerFailure(failure)));
                });
            }
        }
    }

    fn apply_snapshot(self, snapshot: SelectionSnapshotDto, changed_path: Option<&[u8]>) {
        let Some(result) = self.result.get_untracked() else {
            return;
        };
        if !self.session_available.get_untracked()
            || result.session_id != snapshot.session_id
            || snapshot.selection_revision < result.selection_revision
        {
            return;
        }
        let mut updated = result.as_ref().clone();
        updated.selection_revision = snapshot.selection_revision;
        updated.actual_fusion_depth = snapshot.actual_fusion_depth;
        updated.tree = Some(snapshot.tree);
        self.update_collapsed(updated.tree.as_ref(), changed_path);
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
        self.session_available.get_untracked()
            && self.result.get_untracked().is_some_and(|result| {
                result.session_id == session_id && result.selection_revision == revision
            })
    }

    pub(super) fn begin_options_request(self, request_id: u64, path: Vec<u8>) {
        batch(|| {
            self.options.set(None);
            self.options_indicator_visible.set(false);
            self.option_query.set(String::new());
            self.option_active_index.set(0);
            self.active_options.set(Some(request_id));
            self.panel_path.set(Some(path));
        });
    }

    pub(super) fn close_options(self) {
        self.panel_path.set(None);
        self.options.set(None);
        self.options_indicator_visible.set(false);
        self.active_options.set(None);
        self.option_query.set(String::new());
        self.option_active_index.set(0);
    }

    pub(super) fn reveal_search_indicator(self, request_id: u64) {
        if self.searching.try_get_untracked() == Some(true)
            && self.active_search.try_get_untracked() == Some(Some(request_id))
        {
            self.search_indicator_visible.set(true);
        }
    }

    pub(super) fn reveal_options_indicator(self, request_id: u64) {
        if self.active_options.try_get_untracked() == Some(Some(request_id))
            && self.panel_path.try_get_untracked().flatten().is_some()
        {
            self.options_indicator_visible.set(true);
        }
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

#[cfg(target_arch = "wasm32")]
fn schedule_loading_indicator(callback: impl FnOnce() + 'static) {
    use wasm_bindgen::JsCast;

    let callback = wasm_bindgen::closure::Closure::once(callback);
    let Some(window) = web_sys::window() else {
        return;
    };
    if window
        .set_timeout_with_callback_and_timeout_and_arguments_0(
            callback.as_ref().unchecked_ref(),
            LOADING_INDICATOR_DELAY_MS,
        )
        .is_ok()
    {
        callback.forget();
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn schedule_loading_indicator(_callback: impl FnOnce() + 'static) {}

#[derive(Clone)]
pub(super) struct Controller {
    pub(super) state: AppState,
    worker: Arc<Mutex<Option<SendWrapper<WorkerClient>>>>,
    next_request_id: Arc<AtomicU64>,
}

impl Controller {
    pub(super) fn new(state: AppState) -> Self {
        Self {
            state,
            worker: Arc::new(Mutex::new(None)),
            next_request_id: Arc::new(AtomicU64::new(1)),
        }
    }

    pub(super) fn start_worker(&self) {
        let state = self.state;
        let generation = state.begin_worker_start();
        self.worker
            .lock()
            .expect("worker lock must be available")
            .take();
        match WorkerClient::new(
            Arc::new(move |response| state.handle_worker_response(generation, response)),
            Arc::new(move |details| state.handle_worker_error(generation, details)),
        ) {
            Ok(worker) => {
                *self.worker.lock().expect("worker lock must be available") =
                    Some(SendWrapper::new(worker));
            }
            Err(error) => state.handle_worker_error(generation, error),
        }
    }

    fn send(&self, request: WorkerRequest) -> bool {
        let result = self
            .worker
            .lock()
            .expect("worker lock must be available")
            .as_ref()
            .ok_or_else(String::new)
            .and_then(|worker| worker.send(&request));
        if let Err(error) = result {
            self.state
                .handle_worker_error(self.state.worker_generation.get_untracked(), error);
            return false;
        }
        true
    }

    fn next_request(&self) -> u64 {
        self.next_request_id.fetch_add(1, Ordering::Relaxed)
    }

    pub(super) fn search(&self) {
        if self.state.searching.get_untracked() || self.state.selection_busy.get_untracked() {
            return;
        }
        let Some(input) = self.state.current_input() else {
            self.state.error.set(Some(AppError::SelectTarget));
            return;
        };
        if !untrack(|| self.state.worker_ready()) {
            return;
        }
        let request_id = self.next_request();
        self.state.begin_search(request_id);
        if self.send(WorkerRequest::Search { request_id, input }) {
            let state = self.state;
            schedule_loading_indicator(move || state.reveal_search_indicator(request_id));
        } else {
            self.state.active_search.set(None);
            self.state.searching.set(false);
            self.state.search_indicator_visible.set(false);
        }
    }

    pub(super) fn open_options(&self, path: Vec<u8>) {
        if !untrack(|| self.state.can_edit_route()) {
            return;
        }
        let Some(result) = self.state.result.get_untracked() else {
            return;
        };
        let request_id = self.next_request();
        self.state.begin_options_request(request_id, path.clone());
        if self.send(WorkerRequest::GetNodeOptions {
            request_id,
            session_id: result.session_id,
            selection_revision: result.selection_revision,
            path,
        }) {
            let state = self.state;
            schedule_loading_indicator(move || state.reveal_options_indicator(request_id));
        } else {
            self.state.close_options();
        }
    }

    pub(super) fn select_option(&self, option_id: u32, selected: bool) {
        if self.state.selection_busy.get_untracked() {
            return;
        }
        if selected {
            self.state.close_options();
            return;
        }
        if !untrack(|| self.state.can_edit_route()) {
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
        self.state.mutation_path.set(Some(path.clone()));
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
        if !untrack(|| self.state.can_edit_route()) {
            return;
        }
        let Some(result) = self.state.result.get_untracked() else {
            return;
        };
        let request_id = self.next_request();
        self.state.close_options();
        self.state.mutation_path.set(None);
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
        if !untrack(|| self.state.worker_ready()) {
            return;
        }
        self.state.target_query.set(query.clone());
        self.state.target_picker_open.set(true);
        self.state.target_active_index.set(0);
        if self
            .state
            .target
            .get_untracked()
            .is_some_and(|target| self.state.i18n.demon_name_untracked(target) != query.trim())
        {
            self.state.target.set(None);
            self.state.required_skills.set(Vec::new());
            self.state.close_skill_picker();
            self.form_changed();
        }
    }

    pub(super) fn select_target(&self, target: Option<DemonId>) {
        if !untrack(|| self.state.worker_ready()) {
            return;
        }
        if let Some(id) = target
            && !self.state.catalog.get_untracked().is_some_and(|catalog| {
                demon(&catalog, id)
                    .is_some_and(|demon| demon_available(demon, self.state.dlc_settings()))
            })
        {
            return;
        }
        self.state.close_skill_picker();
        self.state.target.set(target);
        self.state.target_query.set(
            target
                .map(|id| self.state.i18n.demon_name_untracked(id))
                .unwrap_or_default(),
        );
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

    pub(super) fn set_locale(&self, locale: Locale) {
        if self.state.i18n.locale_untracked() == locale {
            return;
        }
        batch(|| {
            self.state.close_pickers();
            self.state.i18n.set_locale(locale);
            self.state.target_query.set(
                self.state
                    .target
                    .get_untracked()
                    .map(|id| self.state.i18n.demon_name_untracked(id))
                    .unwrap_or_default(),
            );
        });
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
            self.state.close_skill_picker();
        }
        self.form_changed();
    }

    pub(super) fn set_depth(&self, depth: u32) {
        self.state.max_depth.set(depth.min(MAX_FUSION_DEPTH));
        self.form_changed();
    }

    pub(super) fn open_skill_picker(&self, slot: usize) {
        if untrack(|| self.state.can_edit_skills())
            && (self.state.required_skills.get_untracked().len()..SKILL_CAPACITY).contains(&slot)
        {
            self.state.skill_picker_open.set(true);
        }
    }

    pub(super) fn add_skill(&self, skill_id: SkillId) {
        if !untrack(|| self.state.can_edit_skills())
            || self.state.required_skills.get_untracked().len() >= SKILL_CAPACITY
        {
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
        self.state.close_skill_picker();
        self.form_changed();
    }

    pub(super) fn remove_skill(&self, index: usize) {
        if !untrack(|| self.state.can_edit_skills()) {
            return;
        }
        self.state.required_skills.update(|skills| {
            if index < skills.len() {
                skills.remove(index);
            }
        });
        self.form_changed();
    }

    pub(super) fn clear_form(&self) {
        let restart_worker = self.state.searching.get_untracked()
            || self.state.selection_busy.get_untracked()
            || self.state.active_options.get_untracked().is_some();
        batch(|| {
            self.state.invalidate_requests();
            self.state.close_pickers();
            self.state.result.set(None);
            self.state.collapsed.set(BTreeSet::new());
            self.state.error.set(None);
            self.state.target.set(None);
            self.state.required_skills.set(Vec::new());
            self.state.max_depth.set(DEFAULT_FUSION_DEPTH);
            self.state.target_query.set(String::new());
            if restart_worker {
                self.start_worker();
            }
        });
        save_form(PersistedForm::from_state(self.state));
    }

    fn form_changed(&self) {
        save_form(PersistedForm::from_state(self.state));
    }

    #[cfg(all(test, not(target_arch = "wasm32")))]
    pub(super) fn test_worker(&self) -> super::worker_client::TestWorker {
        self.worker.lock().unwrap().as_ref().unwrap().test_handle()
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum AppError {
    SelectTarget,
    WorkerFailed(String),
    WorkerFailure(WorkerFailureDto),
}

impl AppError {
    pub(super) fn localized(&self, i18n: I18n) -> String {
        match self {
            Self::SelectTarget => i18n.text(Message::SelectTargetError),
            Self::WorkerFailed(details) => i18n.worker_failed(details),
            Self::WorkerFailure(failure) => match failure.code {
                WorkerFailureCode::UnknownDemon => i18n.text(Message::ErrorUnknownDemon),
                WorkerFailureCode::UnknownSkill => i18n.text(Message::ErrorUnknownSkill),
                WorkerFailureCode::UnsupportedSkill => i18n.text(Message::ErrorUnsupportedSkill),
                WorkerFailureCode::TooManySkills => i18n.too_many_skills(
                    failure.selected.unwrap_or_default(),
                    failure.maximum.unwrap_or(SKILL_CAPACITY),
                ),
                WorkerFailureCode::UnavailableTarget => i18n.text(Message::ErrorUnavailableTarget),
                WorkerFailureCode::ExpandedStatesLimit
                | WorkerFailureCode::SkillAssignmentsLimit => i18n.text(Message::ErrorSafetyLimit),
                WorkerFailureCode::InvalidSession | WorkerFailureCode::StaleSelection => {
                    i18n.text(Message::ErrorStaleSelection)
                }
                WorkerFailureCode::InvalidPath | WorkerFailureCode::InvalidOption => {
                    i18n.text(Message::ErrorInvalidOption)
                }
                WorkerFailureCode::NoRoute => i18n.text(Message::NoRoute),
                WorkerFailureCode::InvalidMessage | WorkerFailureCode::Internal => {
                    i18n.text(Message::ErrorInternal)
                }
            },
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
