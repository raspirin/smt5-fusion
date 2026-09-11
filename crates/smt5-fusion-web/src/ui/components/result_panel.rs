use std::{collections::BTreeSet, sync::Arc};

use leptos::prelude::*;

use crate::{i18n::Message, protocol::SearchResultDto};

use super::{
    super::{
        selectors::{all_collapsible_paths, grouped_digits, route_contains_direct_element},
        state::{AppState, Controller},
    },
    route_tree::RouteNode,
};

#[component]
pub(crate) fn ResultPanel() -> impl IntoView {
    let state = expect_context::<Controller>().state;
    let i18n = state.i18n;
    view! {
        <section class="result-panel" aria-label=move || i18n.text(Message::SearchResult)>
            <Show when=move || state.result.get().is_none()>
                <div class="result-placeholder card">
                    <div class="route-glyph" aria-hidden="true">"◇ ─ ◇"</div>
                    <h2>{move || i18n.text(Message::EmptyResultTitle)}</h2>
                    <p>{move || i18n.text(Message::EmptyResultHelp)}</p>
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
    let i18n = state.i18n;
    let reset_controller = controller.clone();
    let route_count = grouped_digits(&result.route_count);
    let tree = result.tree.clone();
    let has_tree = tree.is_some();
    let contains_element = tree.as_ref().is_some_and(|tree| {
        state
            .catalog
            .get_untracked()
            .is_some_and(|catalog| route_contains_direct_element(tree, &catalog))
    });
    let collapse_paths = tree.as_ref().map(all_collapsible_paths).unwrap_or_default();

    view! {
        <div class="result-content">
            <div class="result-summary card">
                <div class="result-summary-layout">
                    <div class="result-total">
                        <span>{move || i18n.text(Message::RouteCount)}</span>
                        <strong class="route-count">{route_count}</strong>
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
                            {move || i18n.text(Message::ExpandAll)}
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
                            {move || i18n.text(Message::CollapseAll)}
                        </button>
                        <button
                            class="button button-secondary"
                            type="button"
                            disabled=move || !state.can_edit_route()
                            on:click=move |_| reset_controller.reset_default()
                        >
                            {move || i18n.text(Message::ResetDefault)}
                        </button>
                    </div>
                </div>
                <Show when=move || {
                    state.result_is_stale()
                        || (!state.session_available.get() && !state.searching.get())
                        || contains_element
                }>
                    <div class="result-notices">
                        <Show when=move || state.result_is_stale()>
                            <p class="result-notice" role="status">
                                {move || i18n.text(Message::StaleResult)}
                            </p>
                        </Show>
                        <Show when=move || !state.session_available.get() && !state.searching.get()>
                            <p class="result-notice" role="status">
                                {move || i18n.text(Message::ResultReadOnly)}
                            </p>
                        </Show>
                        <Show when=move || contains_element>
                            <p class="result-notice" role="status">
                                {move || i18n.text(Message::ElementPriceNote)}
                            </p>
                        </Show>
                    </div>
                </Show>
            </div>
            {match tree {
                Some(tree) => view! {
                    <div class="tree-wrap">
                        <ul
                            class="route-tree"
                            role="tree"
                            aria-label=move || i18n.text(Message::RouteTreeLabel)
                        >
                            <RouteNode node=tree />
                        </ul>
                    </div>
                }.into_any(),
                None => view! {
                    <div class="result-placeholder card no-route" role="status">
                        <h3>{move || i18n.text(Message::NoRoute)}</h3>
                        <p>{move || i18n.text(Message::NoRouteHelp)}</p>
                    </div>
                }.into_any(),
            }}
        </div>
    }
}

fn tree_actions_unavailable(state: AppState, has_tree: bool) -> bool {
    !has_tree || state.selection_busy.get()
}
