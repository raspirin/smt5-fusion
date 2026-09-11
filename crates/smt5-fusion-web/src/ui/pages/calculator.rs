use leptos::prelude::*;

use crate::i18n::{I18n, Message, initial_locale};

use super::super::{
    components::{
        ErrorNotice, LanguageSelector, ResultPanel, SearchPanel, SkillPicker, SourcePicker,
        ThemeSwitcher,
    },
    state::{AppState, Controller, load_form},
    theme::ThemeState,
};

const SOURCE_REPOSITORY_URL: &str = "https://github.com/raspirin/smt5-fusion";
const FEEDBACK_URL: &str = "https://github.com/raspirin/smt5-fusion/issues/new";

fn preview_tape(i18n: I18n) -> impl IntoView {
    #[cfg(feature = "preview")]
    {
        view! {
            <span
                class="preview-tape"
                role="img"
                aria-label=move || i18n.text(Message::PreviewLabel)
            ></span>
        }
    }
    #[cfg(not(feature = "preview"))]
    let _ = i18n;
}

#[component]
pub fn App() -> impl IntoView {
    provide_context(ThemeState::load());
    let i18n = I18n::new(initial_locale());
    let state = AppState::new(load_form(), i18n);
    let controller = Controller::new(state);
    provide_context(controller.clone());
    controller.start_worker();

    view! {
        <a class="skip-link" href="#main-content" inert=move || state.modal_open()>
            {move || i18n.text(Message::SkipToMain)}
        </a>
        <div class="app-shell">
            <header class="site-header" inert=move || state.modal_open()>
                <div class="site-title">
                    <span class="title-ornament" aria-hidden="true"></span>
                    <h1>{move || i18n.text(Message::AppTitle)}</h1>
                </div>
                <div class="header-controls">
                    <LanguageSelector />
                    <ThemeSwitcher />
                </div>
            </header>
            {preview_tape(i18n)}
            <main id="main-content" class="workspace" inert=move || state.modal_open()>
                <SearchPanel />
                <ResultPanel />
            </main>
            <footer class="site-footer" inert=move || state.modal_open()>
                <nav class="footer-links" aria-label=move || i18n.text(Message::ProjectLinks)>
                    <a
                        href=SOURCE_REPOSITORY_URL
                        target="_blank"
                        rel="noopener noreferrer"
                    >
                        {move || i18n.text(Message::SourceRepository)}
                    </a>
                    <span aria-hidden="true">"·"</span>
                    <a href=FEEDBACK_URL target="_blank" rel="noopener noreferrer">
                        {move || i18n.text(Message::SubmitFeedback)}
                    </a>
                </nav>
            </footer>
            <SkillPicker />
            <SourcePicker />
            <ErrorNotice />
        </div>
    }
}
