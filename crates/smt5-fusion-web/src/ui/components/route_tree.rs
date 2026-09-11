use crate::{
    i18n::{Message, skill_category_slug},
    protocol::{RouteTreeNodeDto, SkillId, UpgradeSkillDto},
};
use leptos::prelude::*;
use wasm_bindgen::JsCast;

use super::{
    super::{
        events::focus_current_target,
        selectors::{demon, grouped_digits, skill},
        state::{AppState, Controller},
    },
    acquisition::{AcquisitionKind, MethodLabel},
    level_flow::LevelFlow,
    source_picker::NodeOptionsPopover,
};

const OPTIONS_VIEWPORT_MARGIN: f64 = 16.0;
const OPTIONS_MAX_HEIGHT: f64 = 560.0;

#[derive(Debug, Clone, Copy, PartialEq)]
struct OptionsPlacement {
    opens_upward: bool,
    max_height: f64,
}

#[component]
pub(super) fn RouteNode(node: RouteTreeNodeDto) -> AnyView {
    let controller = expect_context::<Controller>();
    let state = controller.state;
    let i18n = state.i18n;
    let RouteTreeNodeDto {
        path,
        demon: demon_id,
        base_level,
        final_level,
        estimated_macca,
        required_skills,
        upgrade_skills,
        acquisition,
        can_change_recipe,
        children,
    } = node;
    let macca = grouped_digits(&estimated_macca);
    let aria_path = path.clone();
    let option_path = path.clone();
    let popover_path = path.clone();
    let has_children = !children.is_empty();
    let acquisition_kind = AcquisitionKind::from_route(&acquisition);
    let node_class = acquisition_kind.route_card_class();
    let catalog = state.catalog.get_untracked();
    let meta = catalog
        .as_deref()
        .and_then(|catalog| demon(catalog, demon_id));
    let race = meta.map(|meta| meta.race);
    let dlc = meta.map(|meta| meta.content);
    let required_label = if path.is_empty() {
        Message::RequiredSkills
    } else {
        Message::ProvidedSkills
    };
    let options_controller = controller.clone();
    let options_open_upward = RwSignal::new(false);
    let options_max_height = RwSignal::new(OPTIONS_MAX_HEIGHT);
    let collapse_button = has_children.then(|| view! { <CollapseButton path=path.clone() /> });
    let required_view = (!required_skills.is_empty()).then(|| {
        view! {
            <div class="skill-block" role="group" aria-label=move || i18n.text(required_label)>
                {skill_groups(&required_skills, &children).into_iter().map(|(source, skills)| view! {
                    <div class="skill-source-group skill-badges">
                        <span class="meta-label">{move || i18n.text(source)}</span>
                        {skills.into_iter().map(|id| {
                            let learning = (source == Message::OwnSkills)
                                .then(|| skill_learning(id, &upgrade_skills));
                            skill_badge(state, id, learning)
                        }).collect_view()}
                    </div>
                }).collect_view()}
            </div>
        }
    });

    view! {
        <li
            class="route-branch"
            role="treeitem"
            aria-expanded=move || {
                has_children.then(|| !state.collapsed.get().contains(&aria_path))
            }
        >
            <article class=node_class>
                <div class="node-topline">
                    <div class="demon-heading">
                        <span class="race-label">{move || {
                            race.map(|race| i18n.race_name(race))
                                .unwrap_or_else(|| i18n.text(Message::UnknownRace))
                        }}</span>
                        <h3 class="demon-name">{move || i18n.demon_name(demon_id)}</h3>
                        {dlc.and_then(|content| {
                            (content != crate::protocol::DemonContent::Base).then(|| view! {
                                <span class="dlc-badge">{move || i18n.content_name(content)}</span>
                            })
                        })}
                    </div>
                    {collapse_button}
                </div>
                <div class="node-method">
                    <MethodLabel kind=acquisition_kind upgraded={final_level > base_level} />
                    <LevelFlow initial_level=base_level final_level />
                    <span class="route-metrics">
                        <span class="route-metric">{move || i18n.text(Message::Macca)}" "<strong>{macca}</strong></span>
                    </span>
                </div>
                {required_view}
                {can_change_recipe.then(|| view! {
                    <div class="node-action-wrap">
                        <button
                            class="button button-secondary node-action"
                            type="button"
                            disabled=move || !state.can_edit_route()
                            on:click=move |event| {
                                focus_current_target(&event);
                                place_options(
                                    &event,
                                    options_open_upward,
                                    options_max_height,
                                );
                                options_controller.open_options(option_path.clone());
                            }
                        >
                            {move || i18n.text(Message::ChoosePlan)}
                        </button>
                        <Show when=move || state.panel_path.get().as_ref() == Some(&popover_path)>
                            <div
                                class="node-options-backdrop"
                                aria-hidden="true"
                                on:click=move |_| state.close_options()
                            ></div>
                            <NodeOptionsPopover
                                open_upward=options_open_upward
                                max_height=options_max_height
                            />
                        </Show>
                    </div>
                })}
            </article>
            <RouteNodeChildren path nodes=children />
        </li>
    }
    .into_any()
}

fn place_options(
    event: &web_sys::MouseEvent,
    open_upward: RwSignal<bool>,
    max_height: RwSignal<f64>,
) {
    let Some(trigger) = event
        .current_target()
        .and_then(|target| target.dyn_into::<web_sys::Element>().ok())
    else {
        return;
    };
    let Some(viewport_height) = web_sys::window()
        .and_then(|window| window.inner_height().ok())
        .and_then(|height| height.as_f64())
    else {
        return;
    };

    let bounds = trigger.get_bounding_client_rect();
    let placement = options_placement(bounds.top(), bounds.bottom(), viewport_height);
    open_upward.set(placement.opens_upward);
    max_height.set(placement.max_height);
}

fn options_placement(
    trigger_top: f64,
    trigger_bottom: f64,
    viewport_height: f64,
) -> OptionsPlacement {
    let space_above = (trigger_top - OPTIONS_VIEWPORT_MARGIN).max(0.0);
    let space_below = (viewport_height - trigger_bottom - OPTIONS_VIEWPORT_MARGIN).max(0.0);
    let opens_upward = space_above > space_below;
    let available_height = if opens_upward {
        space_above
    } else {
        space_below
    };
    OptionsPlacement {
        opens_upward,
        max_height: available_height.min(OPTIONS_MAX_HEIGHT),
    }
}

#[component]
fn CollapseButton(path: Vec<u8>) -> impl IntoView {
    let state = expect_context::<Controller>().state;
    let i18n = state.i18n;
    let click_path = path.clone();
    let label_path = path.clone();
    view! {
        <button
            type="button"
            class="icon-button collapse-button"
            aria-label=move || i18n.text(Message::ToggleMaterials)
            aria-expanded=move || !state.collapsed.get().contains(&path)
            disabled=move || state.selection_busy.get()
            on:click=move |_| {
                state.close_options();
                state.collapsed.update(|collapsed| {
                    if !collapsed.remove(&click_path) {
                        collapsed.insert(click_path.clone());
                    }
                });
            }
        >
            {move || if state.collapsed.get().contains(&label_path) { "＋" } else { "−" }}
        </button>
    }
}

#[component]
fn RouteNodeChildren(path: Vec<u8>, nodes: Vec<RouteTreeNodeDto>) -> AnyView {
    let state = expect_context::<Controller>().state;
    view! {
        {move || (!nodes.is_empty() && !state.collapsed.get().contains(&path)).then(|| view! {
            <ul class="route-tree source-tree" role="group">
                {nodes.clone().into_iter().map(|child| view! { <RouteNode node=child /> }).collect_view()}
            </ul>
        })}
    }
    .into_any()
}

fn skill_groups(
    required_skills: &[SkillId],
    children: &[RouteTreeNodeDto],
) -> Vec<(Message, Vec<SkillId>)> {
    let (own, inherited): (Vec<_>, Vec<_>) = required_skills.iter().copied().partition(|id| {
        !children
            .iter()
            .any(|child| child.required_skills.contains(id))
    });
    [
        (Message::OwnSkills, own),
        (Message::InheritedSkills, inherited),
    ]
    .into_iter()
    .filter(|(_, skills)| !skills.is_empty())
    .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SkillLearning {
    Initial,
    Level(u32),
}

fn skill_learning(skill_id: SkillId, upgrade_skills: &[UpgradeSkillDto]) -> SkillLearning {
    upgrade_skills
        .iter()
        .find(|learned| learned.skill == skill_id)
        .map_or(SkillLearning::Initial, |learned| {
            SkillLearning::Level(learned.level)
        })
}

fn skill_badge(
    state: AppState,
    skill_id: SkillId,
    learning: Option<SkillLearning>,
) -> impl IntoView {
    let i18n = state.i18n;
    let category = state
        .catalog
        .get_untracked()
        .and_then(|catalog| skill(&catalog, skill_id).map(|skill| skill.category));
    view! {
        <span
            class=category.map(|category| format!("skill-badge kind-{}", skill_category_slug(category))).unwrap_or_else(|| "skill-badge".to_owned())
            title=move || category.map(|category| i18n.skill_category_name(category))
        >
            <strong>{move || i18n.skill_name(skill_id)}</strong>
            {learning.map(|learning| view! {
                <span class="skill-learning">{move || match learning {
                    SkillLearning::Initial => i18n.text(Message::InitialSkill),
                    SkillLearning::Level(level) => format!("Lv.{level}"),
                }}</span>
            })}
        </span>
    }
}

#[cfg(test)]
mod tests {
    use smt5_fusion_core::dataset::{demon_ids, skill_ids};

    use super::{OptionsPlacement, SkillLearning, options_placement, skill_groups, skill_learning};
    use crate::{
        i18n::Message,
        protocol::{
            AcquisitionDto, RouteTreeNodeDto, SearchInputDto, SkillId, WorkerRequest,
            WorkerResponse,
        },
        service::WorkerService,
    };

    fn pixie_tree(skills: &[SkillId], depth: u32) -> RouteTreeNodeDto {
        let response = WorkerService::new().handle(WorkerRequest::Search {
            request_id: 1,
            input: SearchInputDto {
                target: demon_ids::PIXIE,
                required_skills: skills.to_vec(),
                max_fusion_depth: depth,
                dlc: Default::default(),
            },
        });
        let WorkerResponse::SearchCompleted { result, .. } = response else {
            panic!("expected a search result");
        };
        result.tree.expect("Pixie must have a route")
    }

    #[test]
    fn initial_and_level_up_skills_are_provided_by_the_demon_itself() {
        let tree = pixie_tree(&[skill_ids::DIA, skill_ids::RAKUKAJA], 0);
        assert!(matches!(tree.acquisition, AcquisitionDto::Direct { .. }));
        assert!(tree.final_level > tree.base_level);
        assert!(
            tree.upgrade_skills
                .iter()
                .any(|skill| skill.skill == skill_ids::RAKUKAJA)
        );
        assert_eq!(
            skill_groups(&tree.required_skills, &tree.children),
            vec![(
                Message::OwnSkills,
                vec![skill_ids::DIA, skill_ids::RAKUKAJA]
            )]
        );
        assert_eq!(
            skill_learning(skill_ids::DIA, &tree.upgrade_skills),
            SkillLearning::Initial
        );
        assert_eq!(
            skill_learning(skill_ids::RAKUKAJA, &tree.upgrade_skills),
            SkillLearning::Level(4)
        );
    }

    #[test]
    fn fusion_skills_follow_the_current_material_assignments() {
        let tree = pixie_tree(&[skill_ids::DIA, skill_ids::AGI], 1);
        assert!(matches!(tree.acquisition, AcquisitionDto::Fusion { .. }));
        assert_eq!(
            skill_groups(&tree.required_skills, &tree.children),
            vec![
                (Message::OwnSkills, vec![skill_ids::DIA]),
                (Message::InheritedSkills, vec![skill_ids::AGI]),
            ]
        );
        for child in &tree.children {
            let expected = if child.required_skills.is_empty() {
                Vec::new()
            } else {
                vec![(Message::OwnSkills, child.required_skills.clone())]
            };
            assert_eq!(
                skill_groups(&child.required_skills, &child.children),
                expected
            );
        }
    }

    #[test]
    fn skill_groups_omit_empty_sources() {
        assert!(skill_groups(&[], &[]).is_empty());
        let tree = pixie_tree(&[skill_ids::AGI], 1);
        assert_eq!(
            skill_groups(&tree.required_skills, &tree.children),
            vec![(Message::InheritedSkills, vec![skill_ids::AGI])]
        );
    }

    #[test]
    fn skill_groups_preserve_skill_order_within_each_source() {
        let direct = pixie_tree(&[skill_ids::DIA, skill_ids::RAKUKAJA], 0);
        let own = vec![skill_ids::RAKUKAJA, skill_ids::DIA];
        assert_eq!(
            skill_groups(&own, &direct.children),
            vec![(Message::OwnSkills, own)]
        );
        let fusion = pixie_tree(&[skill_ids::AGI, skill_ids::BUFU, skill_ids::DIA], 2);
        assert_eq!(
            skill_groups(
                &[skill_ids::BUFU, skill_ids::DIA, skill_ids::AGI],
                &fusion.children
            ),
            vec![
                (Message::OwnSkills, vec![skill_ids::DIA]),
                (
                    Message::InheritedSkills,
                    vec![skill_ids::BUFU, skill_ids::AGI]
                ),
            ]
        );
    }

    #[test]
    fn source_picker_uses_the_larger_visible_side() {
        assert_eq!(
            options_placement(900.0, 942.0, 1000.0),
            OptionsPlacement {
                opens_upward: true,
                max_height: 560.0,
            }
        );
        assert_eq!(
            options_placement(40.0, 82.0, 1000.0),
            OptionsPlacement {
                opens_upward: false,
                max_height: 560.0,
            }
        );
        assert_eq!(
            options_placement(300.0, 342.0, 600.0),
            OptionsPlacement {
                opens_upward: true,
                max_height: 284.0,
            }
        );
        assert_eq!(
            options_placement(100.0, 142.0, 400.0),
            OptionsPlacement {
                opens_upward: false,
                max_height: 242.0,
            }
        );
    }
}
