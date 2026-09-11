use leptos::prelude::*;

use crate::{
    i18n::Message,
    protocol::{OptionAcquisitionDto, VisibleOptionDto},
};

use super::{
    super::{
        events::focus_picker_on_open,
        selectors::{
            demon, filtered_options, grouped_digits, route_node_at_path, source_active_descendant,
        },
        state::Controller,
    },
    acquisition::{AcquisitionKind, MethodLabel},
    level_flow::LevelFlow,
    picker_dialog::PickerDialog,
};

pub(super) fn recipe_trigger_id(path: &[u8]) -> String {
    let suffix = if path.is_empty() {
        "root".to_owned()
    } else {
        path.iter().map(u8::to_string).collect::<Vec<_>>().join("-")
    };
    format!("recipe-trigger-{suffix}")
}

#[component]
pub(crate) fn SourcePicker() -> impl IntoView {
    let state = expect_context::<Controller>().state;
    view! {
        <Show when=move || state.panel_path.get().is_some()>
            <SourcePickerDialog />
        </Show>
    }
}

#[component]
fn SourcePickerDialog() -> impl IntoView {
    let controller = expect_context::<Controller>();
    let state = controller.state;
    let i18n = state.i18n;
    let keyboard_controller = controller.clone();
    let panel_path = state.panel_path.get_untracked();
    let return_focus_id = panel_path.as_deref().map(recipe_trigger_id);
    let title_demon = panel_path.and_then(|path| {
        state.result.get_untracked().and_then(|result| {
            route_node_at_path(result.tree.as_ref()?, &path).map(|node| node.demon)
        })
    });
    let input_ref = NodeRef::<leptos::html::Input>::new();
    input_ref.on_load(|input| focus_picker_on_open(&input));
    view! {
        <PickerDialog
            heading_id="node-options-heading"
            title=Signal::derive(move || title_demon.map(|demon| {
                i18n.options_title(&i18n.demon_name(demon))
            }).unwrap_or_else(|| i18n.text(Message::ChoosePlan)))
            on_close=Callback::new(move |()| state.close_options())
            return_focus_id
        >
            <input
                class="text-input source-search"
                type="search"
                role="combobox"
                autocomplete="off"
                node_ref=input_ref
                aria-autocomplete="list"
                aria-controls="node-option-list"
                aria-expanded="true"
                aria-activedescendant=move || source_active_descendant(state)
                placeholder=move || i18n.text(Message::FilterMaterial)
                prop:value=move || state.option_query.get()
                on:input=move |event| {
                    state.option_query.set(event_target_value(&event));
                    state.option_active_index.set(0);
                }
                on:keydown=move |event: web_sys::KeyboardEvent| {
                    let options = filtered_options(state);
                    let count = options.len();
                    match event.key().as_str() {
                        "ArrowDown" if count > 0 => {
                            event.prevent_default();
                            state.option_active_index.update(|index| {
                                *index = (*index + 1).min(count - 1);
                            });
                        }
                        "ArrowUp" if count > 0 => {
                            event.prevent_default();
                            state.option_active_index.update(|index| {
                                *index = index.saturating_sub(1).min(count - 1);
                            });
                        }
                        "Enter" => {
                            if let Some(option) = options.get(
                                state.option_active_index.get_untracked(),
                            ) {
                                event.prevent_default();
                                keyboard_controller.select_option(option.option_id, option.selected);
                            }
                        }
                        _ => {}
                    }
                }
            />
            <Show when=move || state.options.get().is_some()>
                <div id="node-option-list" class="picker-list" role="listbox">
                    {move || {
                        let options = filtered_options(state);
                        if options.is_empty() {
                            view! { <p class="empty-list">{i18n.text(Message::NoMatchingPlan)}</p> }.into_any()
                        } else {
                            options.into_iter().enumerate().map(|(index, option)| view! {
                                <OptionCard option index />
                            }).collect_view().into_any()
                        }
                    }}
                </div>
            </Show>
        </PickerDialog>
    }
}

#[component]
fn OptionCard(option: VisibleOptionDto, index: usize) -> impl IntoView {
    let controller = expect_context::<Controller>();
    let state = controller.state;
    let i18n = state.i18n;
    let option_id = option.option_id;
    let selected = option.selected;
    let score = grouped_digits(&option.score.to_string());
    let estimated_macca = grouped_digits(&option.estimated_macca);
    let acquisition_kind = AcquisitionKind::from_option(&option.acquisition);
    let option_class = acquisition_kind.option_card_class();
    let select_controller = controller.clone();
    let metrics = view! {
        <span class="route-metrics">
            <span class="route-metric">{move || i18n.text(Message::Macca)}" "<strong>{estimated_macca}</strong></span>
            <span class="route-metric">{move || i18n.text(Message::Score)}" "<strong>{score}</strong></span>
        </span>
    };
    let content = match option.acquisition {
        OptionAcquisitionDto::Direct {
            summon_level,
            target_level,
        } => view! {
            <div class="option-titleline">
                <MethodLabel kind=acquisition_kind upgraded={target_level > summon_level} />
                <LevelFlow initial_level=summon_level final_level=target_level />
                {metrics}
            </div>
        }.into_any(),
        OptionAcquisitionDto::Fusion {
            is_special: _,
            route_depth,
            fusion_level,
            target_level,
            materials,
        } => view! {
            <div class="option-titleline">
                <MethodLabel kind=acquisition_kind upgraded={target_level > fusion_level} />
                <LevelFlow initial_level=fusion_level final_level=target_level />
                <span class="level-flow">{move || i18n.route_depth(route_depth)}</span>
                {metrics}
            </div>
            <div class="material-list">
                {materials.into_iter().enumerate().map(|(material_index, material)| {
                    let details = state.catalog.get_untracked().and_then(|catalog| {
                        demon(&catalog, material.demon).map(|meta| (meta.race, meta.base_level))
                    });
                    let required_skills = (!material.required_skills.is_empty()).then(|| view! {
                        <div class="compact-skills">
                            {material.required_skills.iter().copied().map(|id| view! {
                                <span>{move || i18n.skill_name(id)}</span>
                            }).collect_view()}
                        </div>
                    });
                    let material_demon = material.demon;
                    view! {
                        <div class="material-entry">
                            {(material_index > 0).then(|| view! {
                                <span class="material-plus" aria-hidden="true">"＋"</span>
                            })}
                            <div class="material-row">
                                <div>
                                    <strong class="demon-name">{move || i18n.demon_name(material_demon)}</strong>
                                    <small>{move || details.map(|(race, level)| {
                                        format!("{} · Lv.{level}", i18n.race_name(race))
                                    }).unwrap_or_default()}</small>
                                    {required_skills}
                                </div>
                            </div>
                        </div>
                    }
                }).collect_view()}
            </div>
        }.into_any(),
    };

    view! {
        <button
            class=option_class
            class:selected
            class:active=move || state.option_active_index.get() == index
            id=format!("source-option-{option_id}")
            type="button"
            role="option"
            aria-selected=selected
            disabled=move || !state.can_edit_route()
            on:mousemove=move |_| state.option_active_index.set(index)
            on:click=move |_| select_controller.select_option(option_id, selected)
        >
            <div class="option-content">
                {content}
            </div>
            <span class="option-action">{move || i18n.text(if selected {
                Message::CurrentPlan
            } else {
                Message::UsePlan
            })}</span>
        </button>
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::recipe_trigger_id;

    #[test]
    fn focus_return_targets_are_distinct_and_stable_for_every_route_path() {
        let paths: &[&[u8]] = &[&[], &[0], &[1], &[0, 1], &[1, 0], &[0, 1, 0]];
        let ids = paths
            .iter()
            .map(|path| recipe_trigger_id(path))
            .collect::<BTreeSet<_>>();
        assert_eq!(ids.len(), paths.len());
        assert_eq!(recipe_trigger_id(&[]), "recipe-trigger-root");
        assert_eq!(recipe_trigger_id(&[0, 1]), "recipe-trigger-0-1");
    }
}
