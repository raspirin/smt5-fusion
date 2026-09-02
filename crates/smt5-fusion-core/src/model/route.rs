use std::rc::Rc;

use super::{demon::DemonId, recipe::RecipeMeta, skill::SkillId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Route {
    Direct {
        demon: DemonId,
    },
    Upgrade {
        level: u32,
        previous: Rc<Route>,
    },
    Fusion {
        recipe: RecipeMeta,
        materials: Vec<FusionSubroute>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FusionSubroute {
    pub required_skills: Vec<SkillId>,
    pub route: Rc<Route>,
}
