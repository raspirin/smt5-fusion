use leptos::prelude::*;

use crate::i18n::{Message, skill_category_slug};

use super::super::{
    selectors::skill,
    state::{Controller, SKILL_CAPACITY},
};

#[component]
pub(super) fn RequiredSkills() -> impl IntoView {
    let i18n = expect_context::<Controller>().state.i18n;
    view! {
        <div class="field-group">
            <span class="field-label">{move || i18n.text(Message::RequiredSkills)}</span>
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
    let i18n = state.i18n;
    let remove_controller = controller.clone();
    let open_controller = controller.clone();
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
                    let localized_skill = i18n.skill_name(skill_id);
                    view! {
                        <div class=slot_class>
                            <button
                                type="button"
                                class="skill-slot-edit"
                                id=format!("skill-slot-{index}")
                                aria-haspopup="dialog"
                                disabled=move || !state.can_edit_skills()
                                on:click={
                                    let controller = open_controller.clone();
                                    move |_| controller.open_skill_picker(index)
                                }
                            >
                                <span class="slot-number">{format!("{:02}", index + 1)}</span>
                                <strong>{localized_skill.clone()}</strong>
                                {category.map(|category| view! {
                                    <span class=format!("skill-kind kind-{}", skill_category_slug(category))>
                                        {i18n.skill_category_name(category)}
                                    </span>
                                })}
                            </button>
                            <button
                                type="button"
                                class="icon-button"
                                aria-label=i18n.remove_skill(&localized_skill)
                                disabled=move || !state.can_edit_skills()
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
                        id=format!("skill-slot-{index}")
                        aria-haspopup="dialog"
                        disabled=move || !state.can_edit_skills()
                        on:click={
                            let controller = open_controller.clone();
                            move |_| controller.open_skill_picker(index)
                        }
                    >
                        <span class="slot-number">{format!("{:02}", index + 1)}</span>
                        <span>{i18n.text(Message::EmptySlot)}</span>
                    </button>
                }.into_any(),
            }
        }}
    }
}
