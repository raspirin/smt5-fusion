use std::collections::BTreeSet;

use leptos::prelude::*;

use crate::{
    i18n::{Locale, skill_category_slug},
    protocol::{
        CatalogDto, DemonCatalogDto, DemonContent, DemonId, DlcSettingsDto, OptionAcquisitionDto,
        RouteTreeNodeDto, SkillCatalogDto, SkillCategory, SkillId, VisibleOptionDto,
    },
};

use super::{
    search::{SearchQuery, SearchScore, demon_search_score, skill_search_score},
    state::AppState,
};

pub(super) fn target_active_descendant(state: AppState) -> String {
    if !state.target_picker_open.get() {
        return String::new();
    }
    let candidates = filtered_demons(state);
    candidates
        .get(state.target_active_index.get())
        .map(|demon| format!("target-option-{}", demon.id.0))
        .unwrap_or_default()
}

pub(super) fn selected_target_index(state: AppState) -> usize {
    let selected = state.target.get_untracked();
    filtered_demons(state)
        .iter()
        .position(|demon| Some(demon.id) == selected)
        .unwrap_or(0)
}

pub(super) fn filtered_demons(state: AppState) -> Vec<DemonCatalogDto> {
    let Some(catalog) = state.catalog.get() else {
        return Vec::new();
    };
    let query = SearchQuery::new(&state.target_query.get());
    let dlc = DlcSettingsDto {
        konohana_sakuya: state.konohana_sakuya_dlc.get(),
        dagda: state.dagda_dlc.get(),
    };
    let selected = state.target.get();
    let locale = state.i18n.locale();
    let mut demons = catalog
        .demons
        .iter()
        .filter(|demon| demon_available(demon, dlc))
        .filter_map(|demon| {
            let score = if query.is_empty() {
                None
            } else {
                Some(demon_search_score(demon.id, &query, locale)?)
            };
            Some((demon.clone(), score))
        })
        .collect::<Vec<_>>();
    demons.sort_unstable_by_key(|(demon, score)| {
        (
            selected != Some(demon.id),
            *score,
            demon.base_level,
            demon.id,
        )
    });
    demons.into_iter().map(|(demon, _)| demon).collect()
}

pub(super) fn filtered_skills(state: AppState) -> Vec<SkillCatalogDto> {
    let (Some(catalog), Some(target_id)) = (state.catalog.get(), state.target.get()) else {
        return Vec::new();
    };
    let Some(target) = demon(&catalog, target_id) else {
        return Vec::new();
    };
    let selected = state.required_skills.get();
    let query = SearchQuery::new(&state.skill_query.get());
    let category = state.skill_category.get();
    let locale = state.i18n.locale();
    let mut skills = catalog
        .skills
        .iter()
        .filter(|skill| skill_eligible_for_target(target, skill))
        .filter(|skill| !selected.contains(&skill.id))
        .filter(|skill| category.is_none_or(|category| skill.category == category))
        .filter_map(|skill| {
            let score = if query.is_empty() {
                None
            } else {
                Some(skill_search_score(skill.id, &query, locale)?)
            };
            Some((skill.clone(), score))
        })
        .collect::<Vec<_>>();
    skills.sort_unstable_by_key(|(skill, score)| (*score, skill.id));
    skills.into_iter().map(|(skill, _)| skill).collect()
}

pub(super) fn source_active_descendant(state: AppState) -> String {
    filtered_options(state)
        .get(state.option_active_index.get())
        .map(|option| format!("source-option-{}", option.option_id))
        .unwrap_or_default()
}

pub(super) fn filtered_options(state: AppState) -> Vec<VisibleOptionDto> {
    let Some(options) = state.options.get() else {
        return Vec::new();
    };
    let query = SearchQuery::new(&state.option_query.get());
    if query.is_empty() {
        return options.options.clone();
    }
    let locale = state.i18n.locale();
    let mut matches = options
        .options
        .iter()
        .filter_map(|option| {
            option_search_score(option, &query, locale).map(|score| (option, score))
        })
        .collect::<Vec<_>>();
    matches.sort_by_key(|(_, score)| *score);
    matches
        .into_iter()
        .map(|(option, _)| option.clone())
        .collect()
}

pub(super) fn skill_eligible_for_target(target: &DemonCatalogDto, skill: &SkillCatalogDto) -> bool {
    skill.supported && (skill.inheritable || target.natural_skills.contains(&skill.id))
}

#[cfg(test)]
pub(super) fn option_matches(option: &VisibleOptionDto, query: &str, locale: Locale) -> bool {
    let query = SearchQuery::new(query);
    query.is_empty() || option_search_score(option, &query, locale).is_some()
}

fn option_search_score(
    option: &VisibleOptionDto,
    query: &SearchQuery,
    locale: Locale,
) -> Option<SearchScore> {
    match &option.acquisition {
        OptionAcquisitionDto::Direct { .. } => None,
        OptionAcquisitionDto::Fusion { materials, .. } => materials
            .iter()
            .filter_map(|material| demon_search_score(material.demon, query, locale))
            .min(),
    }
}

pub(super) fn all_collapsible_paths(tree: &RouteTreeNodeDto) -> BTreeSet<Vec<u8>> {
    fn visit(node: &RouteTreeNodeDto, paths: &mut BTreeSet<Vec<u8>>) {
        if !node.children.is_empty() {
            paths.insert(node.path.clone());
        }
        for child in &node.children {
            visit(child, paths);
        }
    }

    let mut paths = BTreeSet::new();
    visit(tree, &mut paths);
    paths
}

pub(super) fn default_collapsed(tree: Option<&RouteTreeNodeDto>) -> BTreeSet<Vec<u8>> {
    fn visit(node: &RouteTreeNodeDto, collapsed: &mut BTreeSet<Vec<u8>>) {
        if node.path.len() >= 2 && !node.children.is_empty() {
            collapsed.insert(node.path.clone());
        }
        for child in &node.children {
            visit(child, collapsed);
        }
    }
    let mut collapsed = BTreeSet::new();
    if let Some(tree) = tree {
        visit(tree, &mut collapsed);
    }
    collapsed
}

pub(super) fn grouped_digits(value: &str) -> String {
    let mut grouped = String::with_capacity(value.len() + value.len() / 3);
    for (index, character) in value.chars().enumerate() {
        if index > 0 && (value.len() - index).is_multiple_of(3) {
            grouped.push(',');
        }
        grouped.push(character);
    }
    grouped
}

pub(super) fn demon(catalog: &CatalogDto, id: DemonId) -> Option<&DemonCatalogDto> {
    catalog
        .demons
        .binary_search_by_key(&id, |demon| demon.id)
        .ok()
        .and_then(|index| catalog.demons.get(index))
}

pub(super) fn skill(catalog: &CatalogDto, id: SkillId) -> Option<&SkillCatalogDto> {
    catalog
        .skills
        .binary_search_by_key(&id, |skill| skill.id)
        .ok()
        .and_then(|index| catalog.skills.get(index))
}

pub(super) fn demon_available(demon: &DemonCatalogDto, dlc: DlcSettingsDto) -> bool {
    match demon.content {
        DemonContent::Base => true,
        DemonContent::KonohanaSakuyaDlc => dlc.konohana_sakuya,
        DemonContent::DagdaDlc => dlc.dagda,
    }
}

pub(super) fn skill_categories() -> [SkillCategory; 13] {
    [
        SkillCategory::Physical,
        SkillCategory::Fire,
        SkillCategory::Ice,
        SkillCategory::Electric,
        SkillCategory::Force,
        SkillCategory::Light,
        SkillCategory::Dark,
        SkillCategory::Almighty,
        SkillCategory::Ailment,
        SkillCategory::Recovery,
        SkillCategory::Support,
        SkillCategory::Passive,
        SkillCategory::Special,
    ]
}

pub(super) fn category_code(category: SkillCategory) -> &'static str {
    skill_category_slug(category)
}

pub(super) fn category_from_code(value: &str) -> Option<SkillCategory> {
    skill_categories()
        .into_iter()
        .find(|category| category_code(*category) == value)
}
