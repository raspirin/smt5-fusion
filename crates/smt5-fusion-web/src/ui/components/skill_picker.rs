use crate::{
    i18n::{Message, skill_category_slug},
    protocol::SkillCatalogDto,
};
use leptos::prelude::*;

use super::super::{
    events::focus_picker_on_open,
    selectors::{category_code, category_from_code, filtered_skills, skill_categories},
    state::Controller,
};
use super::picker_dialog::PickerDialog;

#[component]
pub(crate) fn SkillPicker() -> impl IntoView {
    let state = expect_context::<Controller>().state;
    view! {
        <Show when=move || state.skill_picker_open.get() && state.can_edit_skills()>
            <SkillPickerDialog />
        </Show>
    }
}

#[component]
fn SkillPickerDialog() -> impl IntoView {
    let state = expect_context::<Controller>().state;
    let i18n = state.i18n;
    let return_focus_id = state
        .skill_picker_slot
        .get_untracked()
        .map(|slot| format!("skill-slot-{slot}"));
    let input_ref = NodeRef::<leptos::html::Input>::new();
    input_ref.on_load(|input| focus_picker_on_open(&input));
    view! {
            <PickerDialog
                heading_id="skill-picker-heading"
                title=Signal::derive(move || i18n.text(Message::SkillPickerTitle))
                on_close=Callback::new(move |()| state.close_skill_picker())
                return_focus_id
            >
                    <div class="skill-picker-filters">
                        <input
                            class="text-input"
                            type="search"
                            node_ref=input_ref
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
                    <div class="picker-list">
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
            </PickerDialog>
    }
}

#[component]
fn SkillPickerOption(skill: SkillCatalogDto) -> impl IntoView {
    let controller = expect_context::<Controller>();
    let state = controller.state;
    let i18n = state.i18n;
    let skill_id = skill.id;
    let category = skill.category;
    let current = move || {
        state.skill_picker_slot.get().is_some_and(|slot| {
            state
                .required_skills
                .with(|skills| skills.get(slot) == Some(&skill_id))
        })
    };
    let option_class = format!(
        "picker-item skill-option kind-{}",
        skill_category_slug(category)
    );
    view! {
        <button
            class=option_class
            type="button"
            aria-current=move || current().then_some("true")
            disabled=move || !state.can_edit_skills()
            on:click=move |_| controller.select_skill(skill_id)
        >
            <span class="skill-option-copy">
                <strong>{move || i18n.skill_name(skill_id)}</strong>
                <small>{move || i18n.skill_category_name(category)}</small>
            </span>
        </button>
    }
}
