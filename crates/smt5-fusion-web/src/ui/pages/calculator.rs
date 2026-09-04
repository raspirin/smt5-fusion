use leptos::prelude::*;

use crate::i18n::text;

use super::super::{
    components::{ErrorNotice, ResultPanel, SearchPanel, SkillPicker},
    state::{AppState, Controller, load_form},
};

#[component]
pub fn App() -> impl IntoView {
    let state = AppState::new(load_form());
    let controller = Controller::new(state);
    provide_context(controller.clone());
    controller.start_worker();

    view! {
        <a class="skip-link" href="#main-content">{text::SKIP_TO_MAIN}</a>
        <div class="app-shell">
            <header class="site-header">
                <div>
                    <span class="title-ornament" aria-hidden="true"></span>
                    <h1>{text::APP_TITLE}</h1>
                </div>
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
