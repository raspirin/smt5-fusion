mod zh_cn;

use crate::protocol::{DemonContent, DemonId, Race, SkillCategory, SkillId};

pub fn demon_name(id: DemonId) -> &'static str {
    zh_cn::DEMON_NAMES
        .get(id.0 as usize)
        .copied()
        .unwrap_or("未知恶魔")
}

pub fn skill_name(id: SkillId) -> &'static str {
    zh_cn::SKILL_NAMES
        .get(id.0 as usize)
        .copied()
        .unwrap_or("未知技能")
}

pub const fn race_name(race: Race) -> &'static str {
    match race {
        Race::Herald => "大天使",
        Race::Megami => "女神",
        Race::Avian => "灵鸟",
        Race::Divine => "天使",
        Race::Yoma => "妖魔",
        Race::Vile => "邪神",
        Race::Raptor => "凶鸟",
        Race::Deity => "魔神",
        Race::Wargod => "军神",
        Race::Avatar => "神兽",
        Race::Holy => "圣兽",
        Race::Genma => "幻魔",
        Race::Element(_) => "精灵",
        Race::Fairy => "妖精",
        Race::Beast => "魔兽",
        Race::Jirae => "地灵",
        Race::Fiend => "魔人",
        Race::Jaki => "邪鬼",
        Race::Wilder => "妖兽",
        Race::Fury => "破坏神",
        Race::Lady => "地母神",
        Race::Dragon => "龙神",
        Race::Kishin => "鬼神",
        Race::Kunitsu => "国津神",
        Race::Femme => "鬼女",
        Race::Brute => "妖鬼",
        Race::Fallen => "堕天使",
        Race::Night => "夜魔",
        Race::Snake => "龙王",
        Race::Tyrant => "魔王",
        Race::Drake => "邪龙",
        Race::Haunt => "幽鬼",
        Race::Foul => "外道",
        Race::Enigma => "秘神",
        Race::Uma => "未确认神兽",
        Race::Qadistu => "女魔",
        Race::Devil => "大魔王",
        Race::Primal => "原天使",
        Race::Proto => "原型",
        Race::Panagia => "圣女",
        Race::King => "王",
        Race::Human => "人类",
    }
}

pub const fn skill_category_name(category: SkillCategory) -> &'static str {
    match category {
        SkillCategory::Physical => "物理",
        SkillCategory::Fire => "火炎",
        SkillCategory::Ice => "冰结",
        SkillCategory::Electric => "电击",
        SkillCategory::Force => "冲击",
        SkillCategory::Light => "破魔",
        SkillCategory::Dark => "咒杀",
        SkillCategory::Almighty => "万能",
        SkillCategory::Ailment => "异常",
        SkillCategory::Recovery => "回复",
        SkillCategory::Support => "辅助",
        SkillCategory::Passive => "被动",
        SkillCategory::Special => "特殊",
        SkillCategory::Innate => "特性",
    }
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

pub const fn content_name(content: DemonContent) -> Option<&'static str> {
    match content {
        DemonContent::Base => None,
        DemonContent::KonohanaSakuyaDlc => Some("木花朔耶"),
        DemonContent::DagdaDlc => Some("达格达"),
    }
}

pub mod text {
    pub const SKIP_TO_MAIN: &str = "跳到主要内容";
    pub const APP_TITLE: &str = "真·女神转生 V Vengeance 合体计算器";
    pub const CALCULATOR: &str = "反向合体计算";
    pub const DLC_SETTINGS: &str = "DLC";
    pub const TARGET_DEMON: &str = "目标恶魔";
    pub const TARGET_SEARCH: &str = "搜索恶魔";
    pub const NO_MATCHING_DEMON: &str = "没有匹配的恶魔";
    pub const SELECT_TARGET_ERROR: &str = "请先选择目标恶魔。";
    pub const REQUIRED_SKILLS: &str = "需求技能";
    pub const EMPTY_SLOT: &str = "空技能槽";
    pub const MAX_DEPTH: &str = "最大融合深度";
    pub const SEARCH: &str = "计算";
    pub const CLEAR: &str = "清空";
    pub const LOADING_DATA: &str = "正在加载计算数据…";
    pub const EMPTY_RESULT_TITLE: &str = "在这里查看合体方案";
    pub const EMPTY_RESULT_HELP: &str = "选择目标恶魔和需求技能，然后开始计算。";
    pub const SEARCH_RESULT: &str = "当前方案";
    pub const ROUTE_COUNT: &str = "合法路线总数";
    pub const ROUTE_COUNT_HELP: &str =
        "路线总数包含技能来源、子路线和精确深度不同的全部合法具体路线。";
    pub const ROUTE_TREE_LABEL: &str = "当前合体方案树";
    pub const ACTUAL_DEPTH: &str = "当前方案深度";
    pub const DEPTH_UNIT: &str = "层";
    pub const EXPAND_ALL: &str = "全部展开";
    pub const COLLAPSE_ALL: &str = "全部折叠";
    pub const RESET_DEFAULT: &str = "恢复默认方案";
    pub const STALE_RESULT: &str = "搜索条件已更改，当前仍显示上次结果";
    pub const NO_ROUTE: &str = "没有符合这些条件的合法路线";
    pub const NO_ROUTE_HELP: &str = "可以减少需求技能、提高最大深度或检查 DLC 设置。";
    pub const CHOOSE_PLAN: &str = "更换来源恶魔";
    pub const OPTIONS_TITLE_PREFIX: &str = "更换「";
    pub const OPTIONS_TITLE_SUFFIX: &str = "」的来源恶魔";
    pub const CURRENT_PLAN: &str = "当前来源";
    pub const CLOSE: &str = "关闭";
    pub const DIRECT: &str = "从恶魔全书召唤";
    pub const DIRECT_AND_UPGRADE: &str = "从恶魔全书召唤并升级";
    pub const NORMAL_FUSION: &str = "普通合体";
    pub const SPECIAL_FUSION: &str = "特殊合体";
    pub const INITIAL_STATE: &str = "初始态";
    pub const FINAL_STATE: &str = "最终态";
    pub const LEVEL_UP_SKILLS: &str = "习得技能";
    pub const PROVIDED_SKILLS: &str = "负责技能";
    pub const FILTER_MATERIAL: &str = "搜索恶魔";
    pub const SKILL_PICKER_TITLE: &str = "添加需求技能";
    pub const SKILL_SEARCH: &str = "搜索技能";
    pub const ALL_CATEGORIES: &str = "全部类别";
    pub const SKILL_CATEGORY: &str = "技能类别";
    pub const NO_MATCHING_SKILL: &str = "没有匹配的可选技能";
    pub const NO_MATCHING_PLAN: &str = "没有匹配的来源";
    pub const OPTIONS_LOADING: &str = "正在整理可用来源…";
    pub const USE_PLAN: &str = "使用此来源";
    pub const UNKNOWN_RACE: &str = "未知种族";
    pub const TOGGLE_MATERIALS: &str = "展开或折叠素材";
    pub const WORKER_FAILED: &str = "计算组件加载失败";
    pub const RETRY: &str = "重新加载计算组件";
    pub const ERROR_TITLE: &str = "无法完成操作";
    pub const DISMISS: &str = "知道了";
    pub const ERROR_UNKNOWN_DEMON: &str = "目标恶魔不存在。";
    pub const ERROR_UNKNOWN_SKILL: &str = "选择了不存在的技能。";
    pub const ERROR_UNSUPPORTED_SKILL: &str = "选择了当前不支持的技能。";
    pub const ERROR_UNAVAILABLE_TARGET: &str = "当前 DLC 设置下无法使用目标恶魔。";
    pub const ERROR_SAFETY_LIMIT: &str = "搜索规模超过当前安全上限。请减少技能或降低最大深度。";
    pub const ERROR_STALE_SELECTION: &str = "方案已经更新，请重新打开节点的方案列表。";
    pub const ERROR_INVALID_OPTION: &str = "这个方案已经失效，请重新选择。";
    pub const ERROR_INTERNAL: &str = "计算组件返回了无法识别的结果，请重新加载。";

    pub fn remove_skill(skill: &str) -> String {
        format!("移除技能 {skill}")
    }

    pub fn show_more(remaining: usize) -> String {
        format!("显示更多（还剩 {remaining} 项）")
    }

    pub fn route_depth(depth: u32) -> String {
        format!("深度 {depth}")
    }

    pub fn too_many_skills(selected: usize, maximum: usize) -> String {
        format!("选择了 {selected} 个技能，最多只能保留 {maximum} 个。")
    }

    pub fn worker_failed_with_details(details: &str) -> String {
        format!("{WORKER_FAILED}：{details}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn element_demons_use_the_shared_element_race_name() {
        use crate::protocol::Element;

        for race in [
            Race::Element(Element::Erthys),
            Race::Element(Element::Aeros),
            Race::Element(Element::Aquans),
            Race::Element(Element::Flaemis),
        ] {
            assert_eq!(race_name(race), "精灵");
        }
    }

    #[test]
    fn generated_name_tables_cover_stable_ids() {
        assert_eq!(zh_cn::DEMON_NAMES.len(), 275);
        assert_eq!(zh_cn::SKILL_NAMES.len(), 763);
        assert!(zh_cn::DEMON_NAMES.iter().all(|name| !name.is_empty()));
        assert!(zh_cn::SKILL_NAMES.iter().all(|name| !name.is_empty()));
    }
}
