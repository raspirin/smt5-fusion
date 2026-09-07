use leptos::prelude::*;

use crate::i18n::{I18n, Message, initial_locale};

use super::super::{
    components::{ErrorNotice, LanguageSelector, ResultPanel, SearchPanel, SkillPicker},
    state::{AppState, Controller, load_form},
};

#[component]
pub fn App() -> impl IntoView {
    let i18n = I18n::new(initial_locale());
    let state = AppState::new(load_form(), i18n);
    let controller = Controller::new(state);
    provide_context(controller.clone());
    controller.start_worker();

    view! {
        <a class="skip-link" href="#main-content">
            {move || i18n.text(Message::SkipToMain)}
        </a>
        <div class="app-shell">
            <header class="site-header">
                <div class="site-title">
                    <span class="title-ornament" aria-hidden="true"></span>
                    <h1>{move || i18n.text(Message::AppTitle)}</h1>
                </div>
                <LanguageSelector />
            </header>
            <main id="main-content" class="workspace">
                <SearchPanel />
                <ResultPanel />
            </main>
            <SkillPicker />
            <ErrorNotice />
        </div>
    }
}
