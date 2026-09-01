#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SkillId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SkillCategory {
    Physical,
    Fire,
    Ice,
    Electric,
    Force,
    Light,
    Dark,
    Almighty,
    Ailment,
    Recovery,
    Support,
    Passive,
    Special,
    Innate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SkillFlags {
    pub unique: bool,
    pub magatsuhi: bool,
    pub item_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Skill {
    pub id: SkillId,
    pub category: SkillCategory,
    pub cost: u32,
    pub rank: u32,
    pub inheritable: bool,
    pub flags: SkillFlags,
}
