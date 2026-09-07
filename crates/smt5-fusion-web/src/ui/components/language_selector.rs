use leptos::prelude::*;

use crate::i18n::{Locale, Message};

use super::super::{events::focus_moved_outside, state::Controller};

#[component]
pub(crate) fn LanguageSelector() -> impl IntoView {
    let controller = expect_context::<Controller>();
    let state = controller.state;
    let picker_open = RwSignal::new(false);
    let active_index = RwSignal::new(locale_index(state.i18n.locale_untracked()));
    let keyboard_controller = controller.clone();
    let option_controller = controller.clone();

    view! {
        <div
            class="language-picker"
            on:focusout=move |event: web_sys::FocusEvent| {
                if focus_moved_outside(&event) {
                    picker_open.set(false);
                }
            }
        >
            <button
                id="language-trigger"
                class="language-trigger"
                class:open=move || picker_open.get()
                type="button"
                role="combobox"
                lang=move || state.i18n.locale().tag()
                aria-label=move || state.i18n.text(Message::LanguageSelector)
                aria-haspopup="listbox"
                aria-controls="language-options"
                aria-expanded=move || picker_open.get()
                aria-activedescendant=move || {
                    picker_open
                        .get()
                        .then(|| format!("language-option-{}", active_index.get()))
                }
                on:click=move |_| {
                    if picker_open.get_untracked() {
                        picker_open.set(false);
                    } else {
                        active_index.set(locale_index(state.i18n.locale_untracked()));
                        picker_open.set(true);
                    }
                }
                on:keydown=move |event: web_sys::KeyboardEvent| {
                    let count = Locale::ALL.len();
                    match event.key().as_str() {
                        "ArrowDown" => {
                            event.prevent_default();
                            if picker_open.get_untracked() {
                                active_index.update(|index| {
                                    *index = (*index + 1).min(count - 1);
                                });
                            } else {
                                active_index.set(locale_index(state.i18n.locale_untracked()));
                                picker_open.set(true);
                            }
                        }
                        "ArrowUp" => {
                            event.prevent_default();
                            if picker_open.get_untracked() {
                                active_index.update(|index| *index = index.saturating_sub(1));
                            } else {
                                active_index.set(locale_index(state.i18n.locale_untracked()));
                                picker_open.set(true);
                            }
                        }
                        "Home" if picker_open.get_untracked() => {
                            event.prevent_default();
                            active_index.set(0);
                        }
                        "End" if picker_open.get_untracked() => {
                            event.prevent_default();
                            active_index.set(count - 1);
                        }
                        "Enter" | " " => {
                            event.prevent_default();
                            if picker_open.get_untracked() {
                                if let Some(locale) = Locale::ALL.get(active_index.get_untracked()) {
                                    keyboard_controller.set_locale(*locale);
                                }
                                picker_open.set(false);
                            } else {
                                active_index.set(locale_index(state.i18n.locale_untracked()));
                                picker_open.set(true);
                            }
                        }
                        "Escape" => picker_open.set(false),
                        _ => {}
                    }
                }
            >
                <span>{move || state.i18n.locale().display_name()}</span>
            </button>
            <Show when=move || picker_open.get()>
                <div
                    id="language-options"
                    class="language-options"
                    role="listbox"
                    aria-label=move || state.i18n.text(Message::LanguageSelector)
                >
                    {Locale::ALL.into_iter().enumerate().map(|(index, locale)| {
                        let controller = option_controller.clone();
                        view! {
                            <button
                                id=format!("language-option-{index}")
                                class="language-option"
                                class:active=move || active_index.get() == index
                                class:selected=move || state.i18n.locale() == locale
                                type="button"
                                role="option"
                                tabindex="-1"
                                lang=locale.tag()
                                aria-selected=move || state.i18n.locale() == locale
                                on:mousemove=move |_| active_index.set(index)
                                on:click=move |_| {
                                    controller.set_locale(locale);
                                    active_index.set(index);
                                    picker_open.set(false);
                                }
                            >
                                <span>{locale.display_name()}</span>
                            </button>
                        }
                    }).collect_view()}
                </div>
            </Show>
        </div>
    }
}

fn locale_index(locale: Locale) -> usize {
    Locale::ALL
        .iter()
        .position(|candidate| *candidate == locale)
        .unwrap_or(0)
}
