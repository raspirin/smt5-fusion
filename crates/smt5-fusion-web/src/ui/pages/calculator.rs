use leptos::prelude::*;

use crate::i18n::{I18n, Message, initial_locale};

use super::super::{
    components::{
        ErrorNotice, LanguageSelector, ResultPanel, SearchPanel, SkillPicker, ThemeSwitcher,
    },
    state::{AppState, Controller, load_form},
    theme::ThemeState,
};

#[component]
pub fn App() -> impl IntoView {
    provide_context(ThemeState::load());
    let i18n = I18n::new(initial_locale());
    let state = AppState::new(load_form(), i18n);
    let controller = Controller::new(state);
    provide_context(controller.clone());
    controller.start_worker();

    view! {
        <a class="skip-link" href="#main-content" inert=move || state.skill_picker_open.get()>
            {move || i18n.text(Message::SkipToMain)}
        </a>
        <div class="app-shell">
            <header class="site-header" inert=move || state.skill_picker_open.get()>
                <div class="site-title">
                    <span class="title-ornament" aria-hidden="true"></span>
                    <h1>{move || i18n.text(Message::AppTitle)}</h1>
                </div>
                <div class="header-controls">
                    <LanguageSelector />
                    <ThemeSwitcher />
                </div>
            </header>
            <main id="main-content" class="workspace" inert=move || state.skill_picker_open.get()>
                <SearchPanel />
                <ResultPanel />
            </main>
            <SkillPicker />
            <ErrorNotice />
        </div>
    }
}
