use super::{race::Race, skill::SkillId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DemonId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DemonContent {
    Base,
    KonohanaSakuyaDlc,
    DagdaDlc,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillAcquisition {
    Initial { order: u32 },
    Level { level: u32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NaturalSkill {
    pub skill: SkillId,
    pub acquisition: SkillAcquisition,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DemonMeta {
    pub id: DemonId,
    pub race: Race,
    pub base_level: u32,
    pub content: DemonContent,
    pub compendium_price: u32,
    pub natural_skills: Vec<NaturalSkill>,
    pub innate_skill: SkillId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Demon {
    pub meta: DemonId,
    pub level: u32,
    pub skills: Vec<SkillId>,
}

impl Demon {
    pub const MAX_SKILLS: usize = 8;
}
