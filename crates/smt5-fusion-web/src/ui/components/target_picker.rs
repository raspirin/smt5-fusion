use leptos::prelude::*;
use wasm_bindgen::JsCast;

use crate::i18n::{demon_name, race_name, text};

use super::super::{
    events::focus_moved_outside,
    selectors::{filtered_demons, selected_target_index, target_active_descendant},
    state::Controller,
};

#[component]
pub(super) fn TargetPicker() -> impl IntoView {
    let controller = expect_context::<Controller>();
    let state = controller.state;
    let query_controller = controller.clone();
    let keyboard_controller = controller.clone();

    view! {
        <div class="field-group">
            <label for="target-query">{text::TARGET_DEMON}</label>
            <div
                class="target-combobox"
                on:focusout=move |event: web_sys::FocusEvent| {
                    if focus_moved_outside(&event) {
                        state.target_picker_open.set(false);
                    }
                }
            >
                <input
                    id="target-query"
                    class="text-input target-input"
                    type="search"
                    role="combobox"
                    autocomplete="off"
                    spellcheck="false"
                    aria-autocomplete="list"
                    aria-controls="target-options"
                    aria-expanded=move || state.target_picker_open.get()
                    aria-activedescendant=move || target_active_descendant(state)
                    placeholder={text::TARGET_SEARCH}
                    prop:value=move || state.target_query.get()
                    disabled=move || !state.worker_ready.get()
                    on:focus=move |event: web_sys::FocusEvent| {
                        if let Some(input) = event
                            .target()
                            .and_then(|target| target.dyn_into::<web_sys::HtmlInputElement>().ok())
                        {
                            input.select();
                        }
                        state.target_picker_open.set(true);
                        state.target_active_index.set(selected_target_index(state));
                    }
                    on:input=move |event| {
                        query_controller.update_target_query(event_target_value(&event));
                    }
                    on:keydown=move |event: web_sys::KeyboardEvent| {
                        let candidates = filtered_demons(state);
                        let count = candidates.len();
                        match event.key().as_str() {
                            "ArrowDown" if count > 0 => {
                                event.prevent_default();
                                state.target_picker_open.set(true);
                                state.target_active_index.update(|index| {
                                    *index = (*index + 1).min(count - 1);
                                });
                            }
                            "ArrowUp" if count > 0 => {
                                event.prevent_default();
                                state.target_picker_open.set(true);
                                state.target_active_index.update(|index| {
                                    *index = index.saturating_sub(1);
                                });
                            }
                            "Enter" if state.target_picker_open.get_untracked() => {
                                if let Some(demon) = candidates.get(
                                    state.target_active_index.get_untracked().min(count.saturating_sub(1)),
                                ) {
                                    event.prevent_default();
                                    keyboard_controller.select_target(Some(demon.id));
                                }
                            }
                            "Escape" => state.target_picker_open.set(false),
                            _ => {}
                        }
                    }
                />
                <Show when=move || state.target_picker_open.get() && state.worker_ready.get()>
                    <div id="target-options" class="target-options" role="listbox">
                        {move || {
                            let controller = expect_context::<Controller>();
                            let candidates = filtered_demons(state);
                            if candidates.is_empty() {
                                view! { <p class="empty-targets">{text::NO_MATCHING_DEMON}</p> }.into_any()
                            } else {
                                candidates
                                    .into_iter()
                                    .enumerate()
                                    .map(|(index, demon)| {
                                        let select_controller = controller.clone();
                                        let demon_id = demon.id;
                                        view! {
                                            <button
                                                id=format!("target-option-{}", demon_id.0)
                                                class="target-option"
                                                class:active=move || state.target_active_index.get() == index
                                                type="button"
                                                role="option"
                                                aria-selected=move || state.target.get() == Some(demon_id)
                                                on:mousemove=move |_| state.target_active_index.set(index)
                                                on:click=move |_| select_controller.select_target(Some(demon_id))
                                            >
                                                <strong class="demon-name">{demon_name(demon_id)}</strong>
                                                <span>{format!("Lv.{} · {}", demon.base_level, race_name(demon.race))}</span>
                                            </button>
                                        }
                                    })
                                    .collect_view()
                                    .into_any()
                            }
                        }}
                    </div>
                </Show>
            </div>
        </div>
    }
}
