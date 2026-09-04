use leptos::prelude::*;

use crate::{i18n::text, protocol::MAX_FUSION_DEPTH};

use super::super::state::Controller;

#[component]
pub(super) fn DepthSlider() -> impl IntoView {
    let controller = expect_context::<Controller>();
    let state = controller.state;

    view! {
        <div class="field-group">
            <label class="field-label" for="depth-range">{text::MAX_DEPTH}</label>
            <div class="depth-slider">
                <input
                    id="depth-range"
                    type="range"
                    min="0"
                    max=MAX_FUSION_DEPTH
                    step="1"
                    aria-label={text::MAX_DEPTH}
                    aria-valuetext=move || format!("{} {}", state.max_depth.get(), text::DEPTH_UNIT)
                    prop:value=move || state.max_depth.get().to_string()
                    on:input=move |event| {
                        if let Ok(depth) = event_target_value(&event).parse() {
                            controller.set_depth(depth);
                        }
                    }
                />
                <div class="depth-ticks" aria-hidden="true">
                    {(0..=MAX_FUSION_DEPTH).map(|depth| view! { <span>{depth}</span> }).collect_view()}
                </div>
            </div>
        </div>
    }
}
