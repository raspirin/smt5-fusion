use std::{collections::BTreeSet, sync::Arc};

use leptos::prelude::*;

use crate::{i18n::text, protocol::SearchResultDto};

use super::{
    super::{
        selectors::{all_collapsible_paths, grouped_digits},
        state::{AppState, Controller},
    },
    route_tree::RouteNode,
};

#[component]
pub(crate) fn ResultPanel() -> impl IntoView {
    let state = expect_context::<Controller>().state;
    view! {
        <section class="result-panel" aria-label={text::SEARCH_RESULT}>
            <Show when=move || state.result.get().is_none()>
                <div class="result-placeholder card">
                    <div class="route-glyph" aria-hidden="true">"◇ ─ ◇"</div>
                    <h2>{text::EMPTY_RESULT_TITLE}</h2>
                    <p>{text::EMPTY_RESULT_HELP}</p>
                </div>
            </Show>
            {move || state.result.get().map(|result| view! { <ResultContent result /> })}
        </section>
    }
}

#[component]
fn ResultContent(result: Arc<SearchResultDto>) -> impl IntoView {
    let controller = expect_context::<Controller>();
    let state = controller.state;
    let reset_controller = controller.clone();
    let route_count = grouped_digits(&result.route_count);
    let tree = result.tree.clone();
    let has_tree = tree.is_some();
    let collapse_paths = tree.as_ref().map(all_collapsible_paths).unwrap_or_default();

    view! {
        <div class="result-content">
            <div class="result-summary card">
                <Show when=move || state.dirty.get()>
                    <p class="stale-notice" role="status">{text::STALE_RESULT}</p>
                </Show>
                <div class="result-summary-layout">
                    <div class="result-metrics">
                        <div class="route-total-metric">
                            <span>{text::ROUTE_COUNT}</span>
                            <strong class="route-count">{route_count}</strong>
                            <p class="count-note">{text::ROUTE_COUNT_HELP}</p>
                        </div>
                        <div class="depth-metric">
                            <span>{text::ACTUAL_DEPTH}</span>
                            <strong>{format!("{} {}", result.actual_fusion_depth, text::DEPTH_UNIT)}</strong>
                        </div>
                    </div>
                    <div class="result-actions">
                        <button
                            class="button button-secondary"
                            type="button"
                            disabled=move || tree_actions_unavailable(state, has_tree)
                            on:click=move |_| {
                                state.close_options();
                                state.collapsed.set(BTreeSet::new());
                            }
                        >
                            {text::EXPAND_ALL}
                        </button>
                        <button
                            class="button button-secondary"
                            type="button"
                            disabled=move || tree_actions_unavailable(state, has_tree)
                            on:click={
                                let paths = collapse_paths.clone();
                                move |_| {
                                    state.close_options();
                                    state.collapsed.set(paths.clone());
                                }
                            }
                        >
                            {text::COLLAPSE_ALL}
                        </button>
                        <button
                            class="button button-secondary"
                            type="button"
                            disabled=move || {
                                tree_actions_unavailable(state, has_tree) || !state.worker_ready.get()
                            }
                            on:click=move |_| reset_controller.reset_default()
                        >
                            {text::RESET_DEFAULT}
                        </button>
                    </div>
                </div>
            </div>
            {match tree {
                Some(tree) => view! {
                    <div class="tree-wrap">
                        <ul class="route-tree" role="tree" aria-label={text::ROUTE_TREE_LABEL}>
                            <RouteNode node=tree />
                        </ul>
                    </div>
                }.into_any(),
                None => view! {
                    <div class="result-placeholder card no-route" role="status">
                        <h3>{text::NO_ROUTE}</h3>
                        <p>{text::NO_ROUTE_HELP}</p>
                    </div>
                }.into_any(),
            }}
        </div>
    }
}

fn tree_actions_unavailable(state: AppState, has_tree: bool) -> bool {
    !has_tree || state.selection_busy.get()
}
