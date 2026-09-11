use leptos::prelude::*;

use crate::{
    i18n::Message,
    protocol::{OptionAcquisitionDto, VisibleOptionDto},
};

use super::{
    super::{
        events::{
            active_element, focus_moved_outside, focus_picker_on_open, preserve_picker_focus,
            restore_focus,
        },
        selectors::{demon, filtered_options, grouped_digits, source_active_descendant},
        state::Controller,
    },
    acquisition::{AcquisitionKind, MethodLabel},
    level_flow::LevelFlow,
};

#[component]
pub(super) fn NodeOptionsPopover(
    open_upward: RwSignal<bool>,
    max_height: RwSignal<f64>,
) -> impl IntoView {
    let controller = expect_context::<Controller>();
    let state = controller.state;
    let i18n = state.i18n;
    let keyboard_controller = controller.clone();
    let return_focus = StoredValue::new_local(active_element());
    let input_ref = NodeRef::<leptos::html::Input>::new();
    input_ref.on_load(|input| focus_picker_on_open(&input));
    view! {
        <section
            class="node-options-popover"
            class:opens-upward=move || open_upward.get()
            style=move || format!("max-height: {:.0}px", max_height.get())
            role="dialog"
            tabindex="-1"
            aria-labelledby="node-options-heading"
            on:focusout=move |event: web_sys::FocusEvent| {
                if focus_moved_outside(&event) {
                    state.close_options();
                }
            }
            on:keydown=move |event: web_sys::KeyboardEvent| {
                if event.key() == "Escape" {
                    event.prevent_default();
                    event.stop_propagation();
                    let target = return_focus.get_value();
                    state.close_options();
                    restore_focus(target);
                }
            }
        >
            <div class="node-options-heading">
                <strong id="node-options-heading">
                    {move || state.options.get().map(|options| {
                        i18n.options_title(&i18n.demon_name(options.demon))
                    }).unwrap_or_else(|| i18n.text(Message::ChoosePlan))}
                </strong>
                <button
                    class="icon-button"
                    type="button"
                    aria-label=move || i18n.text(Message::Close)
                    on:mousedown=preserve_picker_focus
                    on:click=move |_| {
                        let target = return_focus.get_value();
                        state.close_options();
                        restore_focus(target);
                    }
                >
                    "×"
                </button>
            </div>
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
            <Show when=move || state.options_indicator_visible.get()>
                <div class="panel-loading" role="status">
                    <span class="spinner" aria-hidden="true"></span>
                    {move || i18n.text(Message::OptionsLoading)}
                </div>
            </Show>
            <Show when=move || state.options.get().is_some()>
                <div id="node-option-list" class="node-option-list" role="listbox">
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
        </section>
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
            on:mousedown=preserve_picker_focus
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
