use std::sync::LazyLock;

#[cfg(not(target_arch = "wasm32"))]
use unicode_normalization::UnicodeNormalization;

use crate::{
    i18n::{Locale, demon_name_aliases, skill_name_aliases},
    protocol::{DemonId, SkillId},
};

static SEARCH_INDEXES: LazyLock<SearchIndexes> = LazyLock::new(SearchIndexes::new);

#[derive(Debug)]
pub(super) struct SearchQuery {
    normalized: NormalizedText,
}

impl SearchQuery {
    pub(super) fn new(value: &str) -> Self {
        Self {
            normalized: normalize(value),
        }
    }

    pub(super) fn is_empty(&self) -> bool {
        self.normalized.compact.is_empty()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct SearchScore {
    quality: MatchQuality,
    locale_priority: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum MatchQuality {
    Exact,
    Prefix,
    WordPrefix,
    Substring,
}

#[derive(Debug)]
struct SearchIndexes {
    demons: Vec<SearchEntry>,
    skills: Vec<SearchEntry>,
}

impl SearchIndexes {
    fn new() -> Self {
        let mut demons = Vec::new();
        while let Some(aliases) = demon_name_aliases(DemonId(demons.len() as u32)) {
            demons.push(SearchEntry::new(aliases));
        }
        let mut skills = Vec::new();
        while let Some(aliases) = skill_name_aliases(SkillId(skills.len() as u32)) {
            skills.push(SearchEntry::new(aliases));
        }
        Self { demons, skills }
    }
}

#[derive(Debug)]
struct SearchEntry {
    aliases: Vec<NormalizedAlias>,
}

impl SearchEntry {
    fn new(aliases: [(Locale, &'static str); 4]) -> Self {
        Self {
            aliases: aliases
                .into_iter()
                .map(|(locale, value)| NormalizedAlias {
                    locale,
                    text: normalize(value),
                })
                .collect(),
        }
    }

    fn score(&self, query: &SearchQuery, current_locale: Locale) -> Option<SearchScore> {
        self.aliases
            .iter()
            .filter_map(|alias| {
                match_quality(&alias.text, &query.normalized).map(|quality| SearchScore {
                    quality,
                    locale_priority: u8::from(alias.locale != current_locale),
                })
            })
            .min()
    }
}

#[derive(Debug)]
struct NormalizedAlias {
    locale: Locale,
    text: NormalizedText,
}

#[derive(Debug, PartialEq, Eq)]
struct NormalizedText {
    spaced: String,
    compact: String,
}

pub(super) fn prepare_search_indexes() {
    LazyLock::force(&SEARCH_INDEXES);
}

pub(super) fn demon_search_score(
    id: DemonId,
    query: &SearchQuery,
    current_locale: Locale,
) -> Option<SearchScore> {
    SEARCH_INDEXES
        .demons
        .get(id.0 as usize)?
        .score(query, current_locale)
}

pub(super) fn skill_search_score(
    id: SkillId,
    query: &SearchQuery,
    current_locale: Locale,
) -> Option<SearchScore> {
    SEARCH_INDEXES
        .skills
        .get(id.0 as usize)?
        .score(query, current_locale)
}

fn normalize(value: &str) -> NormalizedText {
    let mut spaced = String::with_capacity(value.len());
    let mut pending_separator = false;
    for character in compatibility_normalized(value)
        .chars()
        .flat_map(char::to_lowercase)
    {
        let character = fold_hiragana(character);
        if is_search_separator(character) {
            pending_separator = !spaced.is_empty();
        } else {
            if pending_separator {
                spaced.push(' ');
                pending_separator = false;
            }
            spaced.push(character);
        }
    }
    let compact = spaced
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect();
    NormalizedText { spaced, compact }
}

#[cfg(target_arch = "wasm32")]
fn compatibility_normalized(value: &str) -> String {
    String::from(js_sys::JsString::from(value).normalize("NFKC"))
}

#[cfg(not(target_arch = "wasm32"))]
fn compatibility_normalized(value: &str) -> String {
    value.nfkc().collect()
}

fn fold_hiragana(character: char) -> char {
    if matches!(character, '\u{3041}'..='\u{3096}' | '\u{309d}'..='\u{309f}') {
        char::from_u32(u32::from(character) + 0x60).unwrap_or(character)
    } else {
        character
    }
}

fn is_search_separator(character: char) -> bool {
    character.is_whitespace()
        || matches!(
            character,
            '-' | '‐'
                | '‑'
                | '‒'
                | '–'
                | '—'
                | '―'
                | '−'
                | '.'
                | '・'
                | '·'
                | '･'
                | '\''
                | '’'
                | '‘'
                | ':'
                | '：'
                | '/'
                | '／'
                | '_'
        )
}

fn match_quality(alias: &NormalizedText, query: &NormalizedText) -> Option<MatchQuality> {
    [
        text_match_quality(&alias.spaced, &query.spaced),
        text_match_quality(&alias.compact, &query.compact),
    ]
    .into_iter()
    .flatten()
    .min()
}

fn text_match_quality(alias: &str, query: &str) -> Option<MatchQuality> {
    if query.is_empty() {
        return None;
    }
    if alias == query {
        Some(MatchQuality::Exact)
    } else if alias.starts_with(query) {
        Some(MatchQuality::Prefix)
    } else if alias.split_whitespace().any(|word| word.starts_with(query)) {
        Some(MatchQuality::WordPrefix)
    } else if alias.contains(query) {
        Some(MatchQuality::Substring)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_width_case_kana_and_separators() {
        assert_eq!(normalize(" ＳＨＩＶＡ ").compact, "shiva");
        assert_eq!(normalize("ｼｳﾞｧ"), normalize("シヴァ"));
        assert_eq!(normalize("しゔぁ"), normalize("シヴァ"));
        assert_eq!(
            normalize("コノハナ・サクヤ").compact,
            normalize("コノハナ サクヤ").compact
        );
        assert_eq!(
            normalize("Jack-Frost").compact,
            normalize("jack frost").compact
        );
    }

    #[test]
    fn demon_names_match_across_every_supported_language() {
        for query in [
            "Shiva",
            "ＳＨＩＶＡ",
            "シヴァ",
            "しゔぁ",
            "ｼｳﾞｧ",
            "湿婆",
            "濕婆",
        ] {
            assert!(
                demon_search_score(DemonId(193), &SearchQuery::new(query), Locale::ZhCn).is_some(),
                "query did not match: {query}"
            );
        }
    }

    #[test]
    fn skill_names_match_across_every_supported_language() {
        for query in [
            "Riberama",
            "Ｒｉｂｅｒａｍａ",
            "リベラマ",
            "りべらま",
            "ﾘﾍﾞﾗﾏ",
            "利悖拉玛",
            "利悖拉瑪",
        ] {
            assert!(
                skill_search_score(SkillId(745), &SearchQuery::new(query), Locale::ZhTw).is_some(),
                "query did not match: {query}"
            );
        }
    }

    #[test]
    fn exact_and_current_locale_matches_rank_first() {
        let exact =
            demon_search_score(DemonId(193), &SearchQuery::new("濕婆"), Locale::ZhTw).unwrap();
        let prefix =
            demon_search_score(DemonId(193), &SearchQuery::new("濕"), Locale::ZhTw).unwrap();
        assert!(exact < prefix);

        let current_locale =
            demon_search_score(DemonId(193), &SearchQuery::new("濕婆"), Locale::ZhTw).unwrap();
        let other_locale =
            demon_search_score(DemonId(193), &SearchQuery::new("濕婆"), Locale::EnUs).unwrap();
        assert!(current_locale < other_locale);
    }
}
