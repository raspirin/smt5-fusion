use leptos::prelude::*;

use crate::i18n::Message;

use super::super::state::Controller;

#[component]
pub(crate) fn ErrorNotice() -> impl IntoView {
    let state = expect_context::<Controller>().state;
    let i18n = state.i18n;
    view! {
        <Show when=move || state.error.get().is_some()>
            <div class="error-notice" role="alert">
                <div class="error-content">
                    <div class="error-title">{move || i18n.text(Message::ErrorTitle)}</div>
                    <p class="error-message">{move || {
                        state
                            .error
                            .get()
                            .map(|error| error.localized(i18n))
                            .unwrap_or_default()
                    }}</p>
                </div>
                <div class="error-actions">
                    <button class="button button-quiet" type="button" on:click=move |_| state.error.set(None)>
                        {move || i18n.text(Message::Dismiss)}
                    </button>
                </div>
            </div>
        </Show>
    }
}
