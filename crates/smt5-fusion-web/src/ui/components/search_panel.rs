use leptos::prelude::*;

use crate::i18n::text;

use super::{
    super::state::Controller, depth_slider::DepthSlider, dlc_settings::DlcSettings,
    required_skills::RequiredSkills, target_picker::TargetPicker,
};

#[component]
pub(crate) fn SearchPanel() -> impl IntoView {
    let controller = expect_context::<Controller>();
    let state = controller.state;
    let clear_controller = controller.clone();
    let search_controller = controller.clone();

    view! {
        <section class="search-panel card" aria-label={text::CALCULATOR}>
            <DlcSettings />
            <TargetPicker />
            <RequiredSkills />
            <DepthSlider />

            <div class="search-actions">
                <button
                    class="button button-primary search-submit"
                    type="button"
                    disabled=move || !state.worker_ready.get() || state.target.get().is_none()
                    on:click=move |_| search_controller.search()
                >
                    {text::SEARCH}
                </button>
                <button
                    class="button button-secondary"
                    type="button"
                    on:click=move |_| clear_controller.clear_form()
                >
                    {text::CLEAR}
                </button>
            </div>

            <Show when=move || !state.worker_ready.get()>
                <div class="inline-status" role="status">
                    <span class="spinner" aria-hidden="true"></span>
                    {text::LOADING_DATA}
                </div>
            </Show>
        </section>
    }
}
