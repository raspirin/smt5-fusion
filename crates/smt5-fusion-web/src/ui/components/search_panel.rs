use leptos::prelude::*;

use crate::i18n::Message;

use super::{
    super::state::{Controller, WorkerStatus},
    depth_slider::DepthSlider,
    dlc_settings::DlcSettings,
    required_skills::RequiredSkills,
    target_picker::TargetPicker,
};

#[component]
pub(crate) fn SearchPanel() -> impl IntoView {
    let controller = expect_context::<Controller>();
    let state = controller.state;
    let i18n = state.i18n;
    let clear_controller = controller.clone();
    let search_controller = controller.clone();
    let retry_controller = controller.clone();

    view! {
        <section class="search-panel card" aria-label=move || i18n.text(Message::Calculator)>
            <DlcSettings />
            <TargetPicker />
            <RequiredSkills />
            <DepthSlider />

            <div class="search-actions">
                <button
                    class="button button-primary search-submit"
                    type="button"
                    disabled=move || !state.can_search()
                    on:click=move |_| search_controller.search()
                >
                    {move || i18n.text(Message::Search)}
                </button>
                <button
                    class="button button-secondary"
                    type="button"
                    on:click=move |_| clear_controller.clear_form()
                >
                    {move || i18n.text(Message::Clear)}
                </button>
            </div>

            <Show when=move || state.worker_status.get() == WorkerStatus::Loading || state.search_indicator_visible.get()>
                <div class="inline-status" role="status">
                    <span class="spinner" aria-hidden="true"></span>
                    {move || i18n.text(if state.worker_ready() {
                        Message::SearchInProgress
                    } else {
                        Message::LoadingData
                    })}
                </div>
            </Show>
            <Show when=move || state.worker_status.get() == WorkerStatus::Failed>
                <div class="inline-status" role="status">
                    {move || i18n.text(Message::WorkerFailed)}
                    <button class="button button-secondary" type="button" on:click={
                        let controller = retry_controller.clone();
                        move |_| controller.start_worker()
                    }>
                        {move || i18n.text(Message::Retry)}
                    </button>
                </div>
            </Show>
        </section>
    }
}
