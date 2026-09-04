use leptos::prelude::*;

use crate::i18n::text;

use super::super::state::Controller;

#[component]
pub(crate) fn ErrorNotice() -> impl IntoView {
    let state = expect_context::<Controller>().state;
    view! {
        <Show when=move || state.error.get().is_some()>
            <div class="error-notice" role="alert">
                <div>
                    <strong>{text::ERROR_TITLE}</strong>
                    <p>{move || state.error.get().unwrap_or_default()}</p>
                </div>
                <div class="error-actions">
                    <Show when=move || !state.worker_ready.get()>
                        <button class="button button-secondary" type="button" on:click=move |_| {
                            expect_context::<Controller>().start_worker();
                        }>
                            {text::RETRY}
                        </button>
                    </Show>
                    <button class="button button-quiet" type="button" on:click=move |_| state.error.set(None)>
                        {text::DISMISS}
                    </button>
                </div>
            </div>
        </Show>
    }
}
