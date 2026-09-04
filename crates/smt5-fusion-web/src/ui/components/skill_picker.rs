use leptos::prelude::*;

use crate::{
    i18n::{skill_category_name, skill_category_slug, skill_name, text},
    protocol::SkillCatalogDto,
};

use super::super::{
    selectors::{category_code, category_from_code, filtered_skills, skill_categories},
    state::Controller,
};

#[component]
pub(crate) fn SkillPicker() -> impl IntoView {
    let state = expect_context::<Controller>().state;
    view! {
        <Show when=move || state.skill_picker_open.get()>
            <div
                class="modal-backdrop"
                role="presentation"
                on:click=move |_| state.skill_picker_open.set(false)
            >
                <section
                    class="picker-panel"
                    role="dialog"
                    aria-modal="true"
                    aria-labelledby="skill-picker-heading"
                    on:click=move |event: web_sys::MouseEvent| event.stop_propagation()
                    on:keydown=move |event: web_sys::KeyboardEvent| {
                        if event.key() == "Escape" {
                            state.skill_picker_open.set(false);
                        }
                    }
                >
                    <div class="node-options-heading">
                        <strong id="skill-picker-heading">{text::SKILL_PICKER_TITLE}</strong>
                        <button class="icon-button" type="button" aria-label={text::CLOSE} on:click=move |_| state.skill_picker_open.set(false)>"×"</button>
                    </div>
                    <div class="skill-picker-filters">
                        <input
                            class="text-input"
                            type="search"
                            autofocus=true
                            placeholder={text::SKILL_SEARCH}
                            prop:value=move || state.skill_query.get()
                            on:input=move |event| state.skill_query.set(event_target_value(&event))
                        />
                        <select
                            class="select-input"
                            aria-label={text::SKILL_CATEGORY}
                            prop:value=move || state.skill_category.get().map(category_code).unwrap_or("").to_owned()
                            on:change=move |event| state.skill_category.set(category_from_code(&event_target_value(&event)))
                        >
                            <option value="">{text::ALL_CATEGORIES}</option>
                            {skill_categories().into_iter().map(|category| view! {
                                <option value=category_code(category)>{skill_category_name(category)}</option>
                            }).collect_view()}
                        </select>
                    </div>
                    <div class="node-option-list skill-picker-list">
                        {move || {
                            let skills = filtered_skills(state);
                            if skills.is_empty() {
                                view! { <p class="empty-list">{text::NO_MATCHING_SKILL}</p> }.into_any()
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
    let skill_id = skill.id;
    let option_class = format!(
        "picker-item skill-option kind-{}",
        skill_category_slug(skill.category)
    );
    view! {
        <button class=option_class type="button" on:click=move |_| controller.add_skill(skill_id)>
            <span class="skill-option-copy">
                <strong>{skill_name(skill.id)}</strong>
                <small>{skill_category_name(skill.category)}</small>
            </span>
        </button>
    }
}
