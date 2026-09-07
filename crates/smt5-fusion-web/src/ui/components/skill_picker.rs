use leptos::prelude::*;

use crate::{
    i18n::{Message, skill_category_slug},
    protocol::SkillCatalogDto,
};

use super::super::{
    selectors::{category_code, category_from_code, filtered_skills, skill_categories},
    state::Controller,
};

#[component]
pub(crate) fn SkillPicker() -> impl IntoView {
    let state = expect_context::<Controller>().state;
    let i18n = state.i18n;
    view! {
        <Show when=move || state.skill_picker_open.get() && state.can_edit_skills()>
            <div
                class="modal-backdrop"
                role="presentation"
                on:click=move |_| state.close_skill_picker()
            >
                <section
                    class="picker-panel"
                    role="dialog"
                    aria-modal="true"
                    aria-labelledby="skill-picker-heading"
                    on:click=move |event: web_sys::MouseEvent| event.stop_propagation()
                    on:keydown=move |event: web_sys::KeyboardEvent| {
                        if event.key() == "Escape" {
                            state.close_skill_picker();
                        }
                    }
                >
                    <div class="node-options-heading">
                        <strong id="skill-picker-heading">{move || i18n.text(Message::SkillPickerTitle)}</strong>
                        <button
                            class="icon-button"
                            type="button"
                            aria-label=move || i18n.text(Message::Close)
                            on:click=move |_| state.close_skill_picker()
                        >
                            "×"
                        </button>
                    </div>
                    <div class="skill-picker-filters">
                        <input
                            class="text-input"
                            type="search"
                            autofocus=true
                            placeholder=move || i18n.text(Message::SkillSearch)
                            prop:value=move || state.skill_query.get()
                            on:input=move |event| state.skill_query.set(event_target_value(&event))
                        />
                        <select
                            class="select-input"
                            aria-label=move || i18n.text(Message::SkillCategory)
                            prop:value=move || state.skill_category.get().map(category_code).unwrap_or("").to_owned()
                            on:change=move |event| state.skill_category.set(category_from_code(&event_target_value(&event)))
                        >
                            <option value="">{move || i18n.text(Message::AllCategories)}</option>
                            {skill_categories().into_iter().map(|category| view! {
                                <option value=category_code(category)>
                                    {move || i18n.skill_category_name(category)}
                                </option>
                            }).collect_view()}
                        </select>
                    </div>
                    <div class="node-option-list skill-picker-list">
                        {move || {
                            let skills = filtered_skills(state);
                            if skills.is_empty() {
                                view! { <p class="empty-list">{i18n.text(Message::NoMatchingSkill)}</p> }.into_any()
                            } else {
                                skills.into_iter().map(|skill| view! {
                                    <SkillPickerOption skill />
                                }).collect_view().into_any()
                            }
                        }}
                    </div>
                </section>
            </div>
        </Show>
    }
}

#[component]
fn SkillPickerOption(skill: SkillCatalogDto) -> impl IntoView {
    let controller = expect_context::<Controller>();
    let state = controller.state;
    let i18n = state.i18n;
    let skill_id = skill.id;
    let category = skill.category;
    let option_class = format!(
        "picker-item skill-option kind-{}",
        skill_category_slug(category)
    );
    view! {
        <button class=option_class type="button" disabled=move || !state.can_edit_skills() on:click=move |_| controller.add_skill(skill_id)>
            <span class="skill-option-copy">
                <strong>{move || i18n.skill_name(skill_id)}</strong>
                <small>{move || i18n.skill_category_name(category)}</small>
            </span>
        </button>
    }
}
