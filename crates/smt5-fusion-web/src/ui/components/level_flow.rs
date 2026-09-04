use leptos::prelude::*;

#[component]
pub(super) fn LevelFlow(initial_level: u32, final_level: u32) -> impl IntoView {
    view! {
        <span class="level-flow">
            <span>{format!("Lv.{initial_level}")}</span>
            {(final_level > initial_level).then(|| view! {
                <span class="level-arrow" aria-hidden="true">"→"</span>
                <span>{format!("Lv.{final_level}")}</span>
            })}
        </span>
    }
}
