use leptos::prelude::*;

use crate::i18n::{skill_category_name, skill_category_slug, skill_name, text};

use super::super::{
    selectors::skill,
    state::{Controller, SKILL_CAPACITY},
};

#[component]
pub(super) fn RequiredSkills() -> impl IntoView {
    view! {
        <div class="field-group">
            <span class="field-label">{text::REQUIRED_SKILLS}</span>
            <div class="skill-slots">
                {(0..SKILL_CAPACITY).map(|index| view! { <SkillSlot index /> }).collect_view()}
            </div>
        </div>
    }
}

#[component]
fn SkillSlot(index: usize) -> impl IntoView {
    let controller = expect_context::<Controller>();
    let state = controller.state;
    let remove_controller = controller.clone();
    view! {
        {move || {
            let selected = state.required_skills.get();
            match selected.get(index).copied() {
                Some(skill_id) => {
                    let category = state
                        .catalog
                        .get()
                        .and_then(|catalog| skill(&catalog, skill_id).map(|skill| skill.category));
                    let slot_class = category
                        .map(|category| format!("skill-slot filled kind-{}", skill_category_slug(category)))
                        .unwrap_or_else(|| "skill-slot filled".to_owned());
                    view! {
                        <div class=slot_class>
                            <div>
                                <span class="slot-number">{format!("{:02}", index + 1)}</span>
                                <strong>{skill_name(skill_id)}</strong>
                                {category.map(|category| view! {
                                    <span class=format!("skill-kind kind-{}", skill_category_slug(category))>
                                        {skill_category_name(category)}
                                    </span>
                                })}
                            </div>
                            <button
                                type="button"
                                class="icon-button"
                                aria-label=text::remove_skill(skill_name(skill_id))
                                on:click={
                                    let controller = remove_controller.clone();
                                    move |_| controller.remove_skill(index)
                                }
                            >
                                "×"
                            </button>
                        </div>
                    }.into_any()
                }
                None => view! {
                    <button
                        type="button"
                        class="skill-slot empty"
                        disabled=move || state.target.get().is_none()
                        on:click=move |_| state.skill_picker_open.set(true)
                    >
                        <span class="slot-number">{format!("{:02}", index + 1)}</span>
                        <span>{text::EMPTY_SLOT}</span>
                    </button>
                }.into_any(),
            }
        }}
    }
}
