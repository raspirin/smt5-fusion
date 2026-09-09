mod names;

use fluent_bundle::{FluentArgs, FluentBundle, FluentResource};
use leptos::prelude::*;
use unic_langid::LanguageIdentifier;

use crate::protocol::{DemonContent, DemonId, Race, SkillCategory, SkillId};

const EN_US_SOURCE: &str = include_str!("../../locales/en-US.ftl");
const JA_JP_SOURCE: &str = include_str!("../../locales/ja-JP.ftl");
const ZH_CN_SOURCE: &str = include_str!("../../locales/zh-CN.ftl");
const ZH_TW_SOURCE: &str = include_str!("../../locales/zh-TW.ftl");

#[cfg(target_arch = "wasm32")]
use crate::build_info::STORAGE_KEYS;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum Locale {
    EnUs,
    JaJp,
    #[default]
    ZhCn,
    ZhTw,
}

impl Locale {
    pub const ALL: [Self; 4] = [Self::ZhCn, Self::ZhTw, Self::EnUs, Self::JaJp];

    pub const fn tag(self) -> &'static str {
        match self {
            Self::EnUs => "en-US",
            Self::JaJp => "ja-JP",
            Self::ZhCn => "zh-CN",
            Self::ZhTw => "zh-TW",
        }
    }

    pub const fn display_name(self) -> &'static str {
        match self {
            Self::EnUs => "English",
            Self::JaJp => "日本語",
            Self::ZhCn => "简体中文",
            Self::ZhTw => "繁體中文",
        }
    }

    pub fn from_tag(tag: &str) -> Option<Self> {
        match tag {
            "en-US" => Some(Self::EnUs),
            "ja-JP" => Some(Self::JaJp),
            "zh-CN" => Some(Self::ZhCn),
            "zh-TW" => Some(Self::ZhTw),
            _ => None,
        }
    }

    pub fn match_language_tag(tag: &str) -> Option<Self> {
        let normalized = tag.replace('_', "-").to_ascii_lowercase();
        let language = normalized.split('-').next()?;
        match language {
            "en" => Some(Self::EnUs),
            "ja" => Some(Self::JaJp),
            "zh" => {
                let has_hant = normalized.split('-').skip(1).any(|part| part == "hant");
                let has_hans = normalized.split('-').skip(1).any(|part| part == "hans");
                let traditional_region = normalized
                    .split('-')
                    .skip(1)
                    .any(|part| matches!(part, "tw" | "hk" | "mo"));
                Some(if has_hant || (!has_hans && traditional_region) {
                    Self::ZhTw
                } else {
                    Self::ZhCn
                })
            }
            _ => None,
        }
    }

    fn language_id(self) -> LanguageIdentifier {
        self.tag()
            .parse()
            .expect("supported locale must be a valid language identifier")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Message {
    SkipToMain,
    AppTitle,
    PreviewLabel,
    PageTitle,
    PageDescription,
    LanguageSelector,
    ThemeSelector,
    ThemeAuto,
    ThemeDark,
    ThemeLight,
    Calculator,
    DlcSettings,
    TargetDemon,
    TargetSearch,
    NoMatchingDemon,
    SelectTargetError,
    RequiredSkills,
    EmptySlot,
    MaxDepth,
    Search,
    SearchInProgress,
    Clear,
    LoadingData,
    EmptyResultTitle,
    EmptyResultHelp,
    SearchResult,
    RouteCount,
    RouteCountHelp,
    RouteTreeLabel,
    ActualDepth,
    DepthValue,
    ExpandAll,
    CollapseAll,
    ResetDefault,
    StaleResult,
    ResultReadOnly,
    NoRoute,
    NoRouteHelp,
    ChoosePlan,
    OptionsTitle,
    CurrentPlan,
    Close,
    Direct,
    DirectAndUpgrade,
    NormalFusion,
    SpecialFusion,
    InitialState,
    FinalState,
    LevelUpSkills,
    ProvidedSkills,
    FilterMaterial,
    SkillPickerTitle,
    SkillSearch,
    AllCategories,
    SkillCategory,
    NoMatchingSkill,
    NoMatchingPlan,
    OptionsLoading,
    UsePlan,
    UnknownDemon,
    UnknownSkill,
    UnknownRace,
    ToggleMaterials,
    WorkerFailed,
    Retry,
    ErrorTitle,
    Dismiss,
    ErrorUnknownDemon,
    ErrorUnknownSkill,
    ErrorUnsupportedSkill,
    ErrorUnavailableTarget,
    ErrorSafetyLimit,
    ErrorStaleSelection,
    ErrorInvalidOption,
    ErrorInternal,
    RemoveSkill,
    RouteDepth,
    TooManySkills,
    WorkerFailedWithDetails,
    RaceHerald,
    RaceMegami,
    RaceAvian,
    RaceDivine,
    RaceYoma,
    RaceVile,
    RaceRaptor,
    RaceDeity,
    RaceWargod,
    RaceAvatar,
    RaceHoly,
    RaceGenma,
    RaceElement,
    RaceFairy,
    RaceBeast,
    RaceJirae,
    RaceFiend,
    RaceJaki,
    RaceWilder,
    RaceFury,
    RaceLady,
    RaceDragon,
    RaceKishin,
    RaceKunitsu,
    RaceFemme,
    RaceBrute,
    RaceFallen,
    RaceNight,
    RaceSnake,
    RaceTyrant,
    RaceDrake,
    RaceHaunt,
    RaceFoul,
    RaceEnigma,
    RaceUma,
    RaceQadistu,
    RaceDevil,
    RacePrimal,
    RaceProto,
    RacePanagia,
    RaceKing,
    RaceHuman,
    CategoryPhysical,
    CategoryFire,
    CategoryIce,
    CategoryElectric,
    CategoryForce,
    CategoryLight,
    CategoryDark,
    CategoryAlmighty,
    CategoryAilment,
    CategoryRecovery,
    CategorySupport,
    CategoryPassive,
    CategorySpecial,
    CategoryInnate,
    ContentKonohanaSakuya,
    ContentDagda,
}

impl Message {
    pub const ALL: [Self; 137] = [
        Self::SkipToMain,
        Self::AppTitle,
        Self::PreviewLabel,
        Self::PageTitle,
        Self::PageDescription,
        Self::LanguageSelector,
        Self::ThemeSelector,
        Self::ThemeAuto,
        Self::ThemeDark,
        Self::ThemeLight,
        Self::Calculator,
        Self::DlcSettings,
        Self::TargetDemon,
        Self::TargetSearch,
        Self::NoMatchingDemon,
        Self::SelectTargetError,
        Self::RequiredSkills,
        Self::EmptySlot,
        Self::MaxDepth,
        Self::Search,
        Self::SearchInProgress,
        Self::Clear,
        Self::LoadingData,
        Self::EmptyResultTitle,
        Self::EmptyResultHelp,
        Self::SearchResult,
        Self::RouteCount,
        Self::RouteCountHelp,
        Self::RouteTreeLabel,
        Self::ActualDepth,
        Self::DepthValue,
        Self::ExpandAll,
        Self::CollapseAll,
        Self::ResetDefault,
        Self::StaleResult,
        Self::ResultReadOnly,
        Self::NoRoute,
        Self::NoRouteHelp,
        Self::ChoosePlan,
        Self::OptionsTitle,
        Self::CurrentPlan,
        Self::Close,
        Self::Direct,
        Self::DirectAndUpgrade,
        Self::NormalFusion,
        Self::SpecialFusion,
        Self::InitialState,
        Self::FinalState,
        Self::LevelUpSkills,
        Self::ProvidedSkills,
        Self::FilterMaterial,
        Self::SkillPickerTitle,
        Self::SkillSearch,
        Self::AllCategories,
        Self::SkillCategory,
        Self::NoMatchingSkill,
        Self::NoMatchingPlan,
        Self::OptionsLoading,
        Self::UsePlan,
        Self::UnknownDemon,
        Self::UnknownSkill,
        Self::UnknownRace,
        Self::ToggleMaterials,
        Self::WorkerFailed,
        Self::Retry,
        Self::ErrorTitle,
        Self::Dismiss,
        Self::ErrorUnknownDemon,
        Self::ErrorUnknownSkill,
        Self::ErrorUnsupportedSkill,
        Self::ErrorUnavailableTarget,
        Self::ErrorSafetyLimit,
        Self::ErrorStaleSelection,
        Self::ErrorInvalidOption,
        Self::ErrorInternal,
        Self::RemoveSkill,
        Self::RouteDepth,
        Self::TooManySkills,
        Self::WorkerFailedWithDetails,
        Self::RaceHerald,
        Self::RaceMegami,
        Self::RaceAvian,
        Self::RaceDivine,
        Self::RaceYoma,
        Self::RaceVile,
        Self::RaceRaptor,
        Self::RaceDeity,
        Self::RaceWargod,
        Self::RaceAvatar,
        Self::RaceHoly,
        Self::RaceGenma,
        Self::RaceElement,
        Self::RaceFairy,
        Self::RaceBeast,
        Self::RaceJirae,
        Self::RaceFiend,
        Self::RaceJaki,
        Self::RaceWilder,
        Self::RaceFury,
        Self::RaceLady,
        Self::RaceDragon,
        Self::RaceKishin,
        Self::RaceKunitsu,
        Self::RaceFemme,
        Self::RaceBrute,
        Self::RaceFallen,
        Self::RaceNight,
        Self::RaceSnake,
        Self::RaceTyrant,
        Self::RaceDrake,
        Self::RaceHaunt,
        Self::RaceFoul,
        Self::RaceEnigma,
        Self::RaceUma,
        Self::RaceQadistu,
        Self::RaceDevil,
        Self::RacePrimal,
        Self::RaceProto,
        Self::RacePanagia,
        Self::RaceKing,
        Self::RaceHuman,
        Self::CategoryPhysical,
        Self::CategoryFire,
        Self::CategoryIce,
        Self::CategoryElectric,
        Self::CategoryForce,
        Self::CategoryLight,
        Self::CategoryDark,
        Self::CategoryAlmighty,
        Self::CategoryAilment,
        Self::CategoryRecovery,
        Self::CategorySupport,
        Self::CategoryPassive,
        Self::CategorySpecial,
        Self::CategoryInnate,
        Self::ContentKonohanaSakuya,
        Self::ContentDagda,
    ];

    pub const fn id(self) -> &'static str {
        match self {
            Self::SkipToMain => "skip-to-main",
            Self::AppTitle => "app-title",
            Self::PreviewLabel => "preview-label",
            Self::PageTitle => "page-title",
            Self::PageDescription => "page-description",
            Self::LanguageSelector => "language-selector",
            Self::ThemeSelector => "theme-selector",
            Self::ThemeAuto => "theme-auto",
            Self::ThemeDark => "theme-dark",
            Self::ThemeLight => "theme-light",
            Self::Calculator => "calculator",
            Self::DlcSettings => "dlc-settings",
            Self::TargetDemon => "target-demon",
            Self::TargetSearch => "target-search",
            Self::NoMatchingDemon => "no-matching-demon",
            Self::SelectTargetError => "select-target-error",
            Self::RequiredSkills => "required-skills",
            Self::EmptySlot => "empty-slot",
            Self::MaxDepth => "max-depth",
            Self::Search => "search",
            Self::SearchInProgress => "search-in-progress",
            Self::Clear => "clear",
            Self::LoadingData => "loading-data",
            Self::EmptyResultTitle => "empty-result-title",
            Self::EmptyResultHelp => "empty-result-help",
            Self::SearchResult => "search-result",
            Self::RouteCount => "route-count",
            Self::RouteCountHelp => "route-count-help",
            Self::RouteTreeLabel => "route-tree-label",
            Self::ActualDepth => "actual-depth",
            Self::DepthValue => "depth-value",
            Self::ExpandAll => "expand-all",
            Self::CollapseAll => "collapse-all",
            Self::ResetDefault => "reset-default",
            Self::StaleResult => "stale-result",
            Self::ResultReadOnly => "result-read-only",
            Self::NoRoute => "no-route",
            Self::NoRouteHelp => "no-route-help",
            Self::ChoosePlan => "choose-plan",
            Self::OptionsTitle => "options-title",
            Self::CurrentPlan => "current-plan",
            Self::Close => "close",
            Self::Direct => "direct",
            Self::DirectAndUpgrade => "direct-and-upgrade",
            Self::NormalFusion => "normal-fusion",
            Self::SpecialFusion => "special-fusion",
            Self::InitialState => "initial-state",
            Self::FinalState => "final-state",
            Self::LevelUpSkills => "level-up-skills",
            Self::ProvidedSkills => "provided-skills",
            Self::FilterMaterial => "filter-material",
            Self::SkillPickerTitle => "skill-picker-title",
            Self::SkillSearch => "skill-search",
            Self::AllCategories => "all-categories",
            Self::SkillCategory => "skill-category",
            Self::NoMatchingSkill => "no-matching-skill",
            Self::NoMatchingPlan => "no-matching-plan",
            Self::OptionsLoading => "options-loading",
            Self::UsePlan => "use-plan",
            Self::UnknownDemon => "unknown-demon",
            Self::UnknownSkill => "unknown-skill",
            Self::UnknownRace => "unknown-race",
            Self::ToggleMaterials => "toggle-materials",
            Self::WorkerFailed => "worker-failed",
            Self::Retry => "retry",
            Self::ErrorTitle => "error-title",
            Self::Dismiss => "dismiss",
            Self::ErrorUnknownDemon => "error-unknown-demon",
            Self::ErrorUnknownSkill => "error-unknown-skill",
            Self::ErrorUnsupportedSkill => "error-unsupported-skill",
            Self::ErrorUnavailableTarget => "error-unavailable-target",
            Self::ErrorSafetyLimit => "error-safety-limit",
            Self::ErrorStaleSelection => "error-stale-selection",
            Self::ErrorInvalidOption => "error-invalid-option",
            Self::ErrorInternal => "error-internal",
            Self::RemoveSkill => "remove-skill",
            Self::RouteDepth => "route-depth",
            Self::TooManySkills => "too-many-skills",
            Self::WorkerFailedWithDetails => "worker-failed-with-details",
            Self::RaceHerald => "race-herald",
            Self::RaceMegami => "race-megami",
            Self::RaceAvian => "race-avian",
            Self::RaceDivine => "race-divine",
            Self::RaceYoma => "race-yoma",
            Self::RaceVile => "race-vile",
            Self::RaceRaptor => "race-raptor",
            Self::RaceDeity => "race-deity",
            Self::RaceWargod => "race-wargod",
            Self::RaceAvatar => "race-avatar",
            Self::RaceHoly => "race-holy",
            Self::RaceGenma => "race-genma",
            Self::RaceElement => "race-element",
            Self::RaceFairy => "race-fairy",
            Self::RaceBeast => "race-beast",
            Self::RaceJirae => "race-jirae",
            Self::RaceFiend => "race-fiend",
            Self::RaceJaki => "race-jaki",
            Self::RaceWilder => "race-wilder",
            Self::RaceFury => "race-fury",
            Self::RaceLady => "race-lady",
            Self::RaceDragon => "race-dragon",
            Self::RaceKishin => "race-kishin",
            Self::RaceKunitsu => "race-kunitsu",
            Self::RaceFemme => "race-femme",
            Self::RaceBrute => "race-brute",
            Self::RaceFallen => "race-fallen",
            Self::RaceNight => "race-night",
            Self::RaceSnake => "race-snake",
            Self::RaceTyrant => "race-tyrant",
            Self::RaceDrake => "race-drake",
            Self::RaceHaunt => "race-haunt",
            Self::RaceFoul => "race-foul",
            Self::RaceEnigma => "race-enigma",
            Self::RaceUma => "race-uma",
            Self::RaceQadistu => "race-qadistu",
            Self::RaceDevil => "race-devil",
            Self::RacePrimal => "race-primal",
            Self::RaceProto => "race-proto",
            Self::RacePanagia => "race-panagia",
            Self::RaceKing => "race-king",
            Self::RaceHuman => "race-human",
            Self::CategoryPhysical => "category-physical",
            Self::CategoryFire => "category-fire",
            Self::CategoryIce => "category-ice",
            Self::CategoryElectric => "category-electric",
            Self::CategoryForce => "category-force",
            Self::CategoryLight => "category-light",
            Self::CategoryDark => "category-dark",
            Self::CategoryAlmighty => "category-almighty",
            Self::CategoryAilment => "category-ailment",
            Self::CategoryRecovery => "category-recovery",
            Self::CategorySupport => "category-support",
            Self::CategoryPassive => "category-passive",
            Self::CategorySpecial => "category-special",
            Self::CategoryInnate => "category-innate",
            Self::ContentKonohanaSakuya => "content-konohana-sakuya",
            Self::ContentDagda => "content-dagda",
        }
    }
}

struct Bundles {
    en_us: FluentBundle<FluentResource>,
    ja_jp: FluentBundle<FluentResource>,
    zh_cn: FluentBundle<FluentResource>,
    zh_tw: FluentBundle<FluentResource>,
}

impl Bundles {
    fn new() -> Self {
        Self {
            en_us: bundle(Locale::EnUs, EN_US_SOURCE),
            ja_jp: bundle(Locale::JaJp, JA_JP_SOURCE),
            zh_cn: bundle(Locale::ZhCn, ZH_CN_SOURCE),
            zh_tw: bundle(Locale::ZhTw, ZH_TW_SOURCE),
        }
    }

    fn get(&self, locale: Locale) -> &FluentBundle<FluentResource> {
        match locale {
            Locale::EnUs => &self.en_us,
            Locale::JaJp => &self.ja_jp,
            Locale::ZhCn => &self.zh_cn,
            Locale::ZhTw => &self.zh_tw,
        }
    }
}

thread_local! {
    static BUNDLES: Bundles = Bundles::new();
}

fn bundle(locale: Locale, source: &str) -> FluentBundle<FluentResource> {
    let resource = FluentResource::try_new(source.to_owned()).unwrap_or_else(|(_, errors)| {
        panic!("invalid {} Fluent resource: {errors:?}", locale.tag())
    });
    let mut bundle = FluentBundle::new(vec![locale.language_id()]);
    bundle.set_use_isolating(false);
    bundle
        .add_resource(resource)
        .expect("each locale must contain unique message identifiers");
    bundle
}

fn format_message(locale: Locale, message: Message, args: Option<&FluentArgs<'_>>) -> String {
    BUNDLES.with(|bundles| {
        format_from_bundle(bundles.get(locale), message, args).unwrap_or_else(|| {
            format_from_bundle(bundles.get(Locale::ZhCn), message, args)
                .unwrap_or_else(|| message.id().to_owned())
        })
    })
}

fn format_from_bundle(
    bundle: &FluentBundle<FluentResource>,
    message: Message,
    args: Option<&FluentArgs<'_>>,
) -> Option<String> {
    let pattern = bundle.get_message(message.id())?.value()?;
    let mut errors = Vec::new();
    let value = bundle
        .format_pattern(pattern, args, &mut errors)
        .into_owned();
    debug_assert!(
        errors.is_empty(),
        "failed to format {}: {errors:?}",
        message.id()
    );
    Some(value)
}

#[derive(Clone, Copy)]
pub struct I18n {
    locale: RwSignal<Locale>,
}

impl I18n {
    pub fn new(locale: Locale) -> Self {
        let i18n = Self {
            locale: RwSignal::new(locale),
        };
        i18n.sync_document(locale);
        i18n
    }

    pub fn locale(self) -> Locale {
        self.locale.get()
    }

    pub fn locale_untracked(self) -> Locale {
        self.locale.get_untracked()
    }

    pub fn set_locale(self, locale: Locale) {
        if self.locale.get_untracked() != locale {
            self.locale.set(locale);
        }
        save_locale(locale);
        self.sync_document(locale);
    }

    pub fn text(self, message: Message) -> String {
        format_message(self.locale(), message, None)
    }

    pub fn text_untracked(self, message: Message) -> String {
        format_message(self.locale_untracked(), message, None)
    }

    pub fn demon_name(self, id: DemonId) -> String {
        demon_name(self.locale(), id)
    }

    pub fn demon_name_untracked(self, id: DemonId) -> String {
        demon_name(self.locale_untracked(), id)
    }

    pub fn skill_name(self, id: SkillId) -> String {
        skill_name(self.locale(), id)
    }

    pub fn race_name(self, race: Race) -> String {
        self.text(race_message(race))
    }

    pub fn skill_category_name(self, category: SkillCategory) -> String {
        self.text(skill_category_message(category))
    }

    pub fn content_name(self, content: DemonContent) -> Option<String> {
        let message = match content {
            DemonContent::Base => return None,
            DemonContent::KonohanaSakuyaDlc => Message::ContentKonohanaSakuya,
            DemonContent::DagdaDlc => Message::ContentDagda,
        };
        Some(self.text(message))
    }

    pub fn remove_skill(self, skill: &str) -> String {
        let mut args = FluentArgs::new();
        args.set("skill", skill);
        format_message(self.locale(), Message::RemoveSkill, Some(&args))
    }

    pub fn route_depth(self, depth: u32) -> String {
        let mut args = FluentArgs::new();
        args.set("depth", i64::from(depth));
        format_message(self.locale(), Message::RouteDepth, Some(&args))
    }

    pub fn depth_value(self, depth: u32) -> String {
        let mut args = FluentArgs::new();
        args.set("depth", i64::from(depth));
        format_message(self.locale(), Message::DepthValue, Some(&args))
    }

    pub fn too_many_skills(self, selected: usize, maximum: usize) -> String {
        let mut args = FluentArgs::new();
        args.set("selected", selected as i64);
        args.set("maximum", maximum as i64);
        format_message(self.locale(), Message::TooManySkills, Some(&args))
    }

    pub fn options_title(self, demon: &str) -> String {
        let mut args = FluentArgs::new();
        args.set("demon", demon);
        format_message(self.locale(), Message::OptionsTitle, Some(&args))
    }

    pub fn worker_failed(self, details: &str) -> String {
        if details.is_empty() {
            return self.text(Message::WorkerFailed);
        }
        let mut args = FluentArgs::new();
        args.set("details", details);
        format_message(self.locale(), Message::WorkerFailedWithDetails, Some(&args))
    }

    fn sync_document(self, locale: Locale) {
        #[cfg(not(target_arch = "wasm32"))]
        let _ = locale;
        #[cfg(target_arch = "wasm32")]
        {
            let Some(document) = web_sys::window().and_then(|window| window.document()) else {
                return;
            };
            if let Some(root) = document.document_element() {
                let _ = root.set_attribute("lang", locale.tag());
            }
            document.set_title(&format_message(locale, Message::PageTitle, None));
            if let Ok(Some(description)) = document.query_selector("meta[name=description]") {
                let _ = description.set_attribute(
                    "content",
                    &format_message(locale, Message::PageDescription, None),
                );
            }
        }
    }
}

pub fn demon_name(locale: Locale, id: DemonId) -> String {
    let names = match locale {
        Locale::EnUs => &names::en_us::DEMON_NAMES,
        Locale::JaJp => &names::ja_jp::DEMON_NAMES,
        Locale::ZhCn => &names::zh_cn::DEMON_NAMES,
        Locale::ZhTw => &names::zh_tw::DEMON_NAMES,
    };
    names.get(id.0 as usize).map_or_else(
        || format_message(locale, Message::UnknownDemon, None),
        |name| (*name).to_owned(),
    )
}

pub fn skill_name(locale: Locale, id: SkillId) -> String {
    let names = match locale {
        Locale::EnUs => &names::en_us::SKILL_NAMES,
        Locale::JaJp => &names::ja_jp::SKILL_NAMES,
        Locale::ZhCn => &names::zh_cn::SKILL_NAMES,
        Locale::ZhTw => &names::zh_tw::SKILL_NAMES,
    };
    names.get(id.0 as usize).map_or_else(
        || format_message(locale, Message::UnknownSkill, None),
        |name| (*name).to_owned(),
    )
}

pub(crate) fn demon_name_aliases(id: DemonId) -> Option<[(Locale, &'static str); 4]> {
    let index = id.0 as usize;
    Some([
        (Locale::EnUs, *names::en_us::DEMON_NAMES.get(index)?),
        (Locale::JaJp, *names::ja_jp::DEMON_NAMES.get(index)?),
        (Locale::ZhCn, *names::zh_cn::DEMON_NAMES.get(index)?),
        (Locale::ZhTw, *names::zh_tw::DEMON_NAMES.get(index)?),
    ])
}

pub(crate) fn skill_name_aliases(id: SkillId) -> Option<[(Locale, &'static str); 4]> {
    let index = id.0 as usize;
    Some([
        (Locale::EnUs, *names::en_us::SKILL_NAMES.get(index)?),
        (Locale::JaJp, *names::ja_jp::SKILL_NAMES.get(index)?),
        (Locale::ZhCn, *names::zh_cn::SKILL_NAMES.get(index)?),
        (Locale::ZhTw, *names::zh_tw::SKILL_NAMES.get(index)?),
    ])
}

pub const fn skill_category_slug(category: SkillCategory) -> &'static str {
    match category {
        SkillCategory::Physical => "physical",
        SkillCategory::Fire => "fire",
        SkillCategory::Ice => "ice",
        SkillCategory::Electric => "electric",
        SkillCategory::Force => "force",
        SkillCategory::Light => "light",
        SkillCategory::Dark => "dark",
        SkillCategory::Almighty => "almighty",
        SkillCategory::Ailment => "ailment",
        SkillCategory::Recovery => "recovery",
        SkillCategory::Support => "support",
        SkillCategory::Passive => "passive",
        SkillCategory::Special => "special",
        SkillCategory::Innate => "innate",
    }
}

const fn race_message(race: Race) -> Message {
    match race {
        Race::Herald => Message::RaceHerald,
        Race::Megami => Message::RaceMegami,
        Race::Avian => Message::RaceAvian,
        Race::Divine => Message::RaceDivine,
        Race::Yoma => Message::RaceYoma,
        Race::Vile => Message::RaceVile,
        Race::Raptor => Message::RaceRaptor,
        Race::Deity => Message::RaceDeity,
        Race::Wargod => Message::RaceWargod,
        Race::Avatar => Message::RaceAvatar,
        Race::Holy => Message::RaceHoly,
        Race::Genma => Message::RaceGenma,
        Race::Element(_) => Message::RaceElement,
        Race::Fairy => Message::RaceFairy,
        Race::Beast => Message::RaceBeast,
        Race::Jirae => Message::RaceJirae,
        Race::Fiend => Message::RaceFiend,
        Race::Jaki => Message::RaceJaki,
        Race::Wilder => Message::RaceWilder,
        Race::Fury => Message::RaceFury,
        Race::Lady => Message::RaceLady,
        Race::Dragon => Message::RaceDragon,
        Race::Kishin => Message::RaceKishin,
        Race::Kunitsu => Message::RaceKunitsu,
        Race::Femme => Message::RaceFemme,
        Race::Brute => Message::RaceBrute,
        Race::Fallen => Message::RaceFallen,
        Race::Night => Message::RaceNight,
        Race::Snake => Message::RaceSnake,
        Race::Tyrant => Message::RaceTyrant,
        Race::Drake => Message::RaceDrake,
        Race::Haunt => Message::RaceHaunt,
        Race::Foul => Message::RaceFoul,
        Race::Enigma => Message::RaceEnigma,
        Race::Uma => Message::RaceUma,
        Race::Qadistu => Message::RaceQadistu,
        Race::Devil => Message::RaceDevil,
        Race::Primal => Message::RacePrimal,
        Race::Proto => Message::RaceProto,
        Race::Panagia => Message::RacePanagia,
        Race::King => Message::RaceKing,
        Race::Human => Message::RaceHuman,
    }
}

const fn skill_category_message(category: SkillCategory) -> Message {
    match category {
        SkillCategory::Physical => Message::CategoryPhysical,
        SkillCategory::Fire => Message::CategoryFire,
        SkillCategory::Ice => Message::CategoryIce,
        SkillCategory::Electric => Message::CategoryElectric,
        SkillCategory::Force => Message::CategoryForce,
        SkillCategory::Light => Message::CategoryLight,
        SkillCategory::Dark => Message::CategoryDark,
        SkillCategory::Almighty => Message::CategoryAlmighty,
        SkillCategory::Ailment => Message::CategoryAilment,
        SkillCategory::Recovery => Message::CategoryRecovery,
        SkillCategory::Support => Message::CategorySupport,
        SkillCategory::Passive => Message::CategoryPassive,
        SkillCategory::Special => Message::CategorySpecial,
        SkillCategory::Innate => Message::CategoryInnate,
    }
}

#[cfg(target_arch = "wasm32")]
pub fn initial_locale() -> Locale {
    let window = web_sys::window();
    if let Some(stored) = window
        .as_ref()
        .and_then(|window| window.local_storage().ok().flatten())
        .and_then(|storage| storage.get_item(STORAGE_KEYS.locale).ok().flatten())
        .and_then(|tag| Locale::from_tag(&tag))
    {
        return stored;
    }
    if let Some(window) = window {
        for value in window.navigator().languages().iter() {
            if let Some(locale) = value
                .as_string()
                .and_then(|tag| Locale::match_language_tag(&tag))
            {
                return locale;
            }
        }
        if let Some(locale) = window
            .navigator()
            .language()
            .and_then(|tag| Locale::match_language_tag(&tag))
        {
            return locale;
        }
    }
    Locale::ZhCn
}

#[cfg(not(target_arch = "wasm32"))]
pub const fn initial_locale() -> Locale {
    Locale::ZhCn
}

#[cfg(target_arch = "wasm32")]
fn save_locale(locale: Locale) {
    let Some(storage) = web_sys::window().and_then(|window| window.local_storage().ok().flatten())
    else {
        return;
    };
    let _ = storage.set_item(STORAGE_KEYS.locale, locale.tag());
}

#[cfg(not(target_arch = "wasm32"))]
fn save_locale(_locale: Locale) {}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::protocol::Element;

    #[test]
    fn accessible_preview_labels_do_not_change_the_standard_page_titles() {
        for (locale, label, title) in [
            (Locale::ZhCn, "预览版", "SMT5V 合体计算器"),
            (Locale::ZhTw, "預覽版", "SMT5V 反向合體計算器"),
            (Locale::EnUs, "Preview", "SMT5V Fusion Calculator"),
            (
                Locale::JaJp,
                "プレビュー版",
                "SMT5V 逆引き合体シミュレーター",
            ),
        ] {
            assert_eq!(format_message(locale, Message::PreviewLabel, None), label);
            assert_eq!(format_message(locale, Message::PageTitle, None), title);
        }
    }

    #[test]
    fn every_locale_contains_exactly_the_supported_messages() {
        let expected = Message::ALL
            .into_iter()
            .map(Message::id)
            .collect::<BTreeSet<_>>();
        for source in [EN_US_SOURCE, JA_JP_SOURCE, ZH_CN_SOURCE, ZH_TW_SOURCE] {
            assert_eq!(resource_message_ids(source), expected);
        }
    }

    #[test]
    fn every_embedded_resource_parses_and_contains_every_message() {
        BUNDLES.with(|bundles| {
            for locale in Locale::ALL {
                let bundle = bundles.get(locale);
                for message in Message::ALL {
                    assert!(
                        bundle.get_message(message.id()).is_some(),
                        "{} is missing {}",
                        locale.tag(),
                        message.id()
                    );
                }
            }
        });
    }

    #[test]
    fn every_message_formats_with_representative_arguments() {
        let mut args = FluentArgs::new();
        args.set("depth", 2);
        args.set("selected", 9);
        args.set("maximum", 8);
        args.set("demon", "Shiva");
        args.set("skill", "Riberama");
        args.set("details", "worker error");
        for locale in Locale::ALL {
            for message in Message::ALL {
                let formatted = format_message(locale, message, Some(&args));
                assert!(
                    !formatted.is_empty(),
                    "{} is empty in {}",
                    message.id(),
                    locale.tag()
                );
            }
        }
    }

    #[test]
    fn english_plural_rules_are_applied() {
        let mut args = FluentArgs::new();
        args.set("depth", 1);
        assert_eq!(
            format_message(Locale::EnUs, Message::DepthValue, Some(&args)),
            "1 level"
        );
        args.set("depth", 2);
        assert_eq!(
            format_message(Locale::EnUs, Message::DepthValue, Some(&args)),
            "2 levels"
        );
    }

    #[test]
    fn language_tags_match_supported_locales() {
        assert_eq!(
            Locale::ALL,
            [Locale::ZhCn, Locale::ZhTw, Locale::EnUs, Locale::JaJp]
        );
        assert_eq!(Locale::match_language_tag("en-GB"), Some(Locale::EnUs));
        assert_eq!(Locale::match_language_tag("ja_JP"), Some(Locale::JaJp));
        assert_eq!(Locale::match_language_tag("zh-Hans"), Some(Locale::ZhCn));
        assert_eq!(Locale::match_language_tag("zh-Hans-TW"), Some(Locale::ZhCn));
        assert_eq!(Locale::match_language_tag("zh-Hant"), Some(Locale::ZhTw));
        assert_eq!(Locale::match_language_tag("zh_HK"), Some(Locale::ZhTw));
        assert_eq!(Locale::match_language_tag("fr-FR"), None);
        assert_eq!(Locale::from_tag("ja-JP"), Some(Locale::JaJp));
        assert_eq!(Locale::from_tag("zh-TW"), Some(Locale::ZhTw));
        assert_eq!(Locale::from_tag("ja"), None);
    }

    #[test]
    fn element_demons_share_the_localized_element_race_name() {
        for (locale, expected) in [
            (Locale::EnUs, "Element"),
            (Locale::JaJp, "精霊"),
            (Locale::ZhCn, "精灵"),
            (Locale::ZhTw, "精靈"),
        ] {
            for race in [
                Race::Element(Element::Erthys),
                Race::Element(Element::Aeros),
                Race::Element(Element::Aquans),
                Race::Element(Element::Flaemis),
            ] {
                assert_eq!(format_message(locale, race_message(race), None), expected);
            }
        }
    }

    #[test]
    fn generated_name_tables_cover_stable_ids() {
        for names in [
            &names::en_us::DEMON_NAMES,
            &names::ja_jp::DEMON_NAMES,
            &names::zh_cn::DEMON_NAMES,
            &names::zh_tw::DEMON_NAMES,
        ] {
            assert_eq!(names.len(), 275);
            assert!(names.iter().all(|name| !name.is_empty()));
        }
        for names in [
            &names::en_us::SKILL_NAMES,
            &names::ja_jp::SKILL_NAMES,
            &names::zh_cn::SKILL_NAMES,
            &names::zh_tw::SKILL_NAMES,
        ] {
            assert_eq!(names.len(), 763);
            assert!(names.iter().all(|name| !name.is_empty()));
        }
    }

    #[test]
    fn representative_names_follow_the_selected_locale() {
        assert_eq!(demon_name(Locale::EnUs, DemonId(193)), "Shiva");
        assert_eq!(demon_name(Locale::JaJp, DemonId(193)), "シヴァ");
        assert_eq!(demon_name(Locale::ZhCn, DemonId(193)), "湿婆");
        assert_eq!(demon_name(Locale::ZhTw, DemonId(193)), "濕婆");
        assert_eq!(skill_name(Locale::EnUs, SkillId(745)), "Riberama");
        assert_eq!(skill_name(Locale::JaJp, SkillId(745)), "リベラマ");
        assert_eq!(skill_name(Locale::ZhCn, SkillId(745)), "利悖拉玛");
        assert_eq!(skill_name(Locale::ZhTw, SkillId(745)), "利悖拉瑪");
        assert_eq!(demon_name(Locale::ZhTw, DemonId(254)), "摩利支天");
        assert_eq!(skill_name(Locale::ZhTw, SkillId(222)), "魔緣號角");
        assert_eq!(skill_name(Locale::ZhTw, SkillId(631)), "破曉的威光");
        assert_eq!(skill_name(Locale::ZhTw, SkillId(669)), "無相幻射");
    }

    fn resource_message_ids(source: &str) -> BTreeSet<&str> {
        source
            .lines()
            .filter_map(|line| {
                if line.starts_with(|character: char| character.is_ascii_lowercase()) {
                    line.split_once('=').map(|(id, _)| id.trim())
                } else {
                    None
                }
            })
            .collect()
    }
}
