use super::{
    demon_catalog::DemonCatalog, skill_catalog::SkillCatalog,
    special_recipe_catalog::SpecialRecipeCatalog,
};

#[derive(Debug)]
pub struct GameData {
    demons: DemonCatalog,
    skills: SkillCatalog,
    special_recipes: SpecialRecipeCatalog,
}

impl GameData {
    pub(crate) fn new(
        demons: DemonCatalog,
        skills: SkillCatalog,
        special_recipes: SpecialRecipeCatalog,
    ) -> Self {
        Self {
            demons,
            skills,
            special_recipes,
        }
    }

    pub fn demons(&self) -> &DemonCatalog {
        &self.demons
    }

    pub fn skills(&self) -> &SkillCatalog {
        &self.skills
    }

    pub(crate) fn special_recipes(&self) -> &SpecialRecipeCatalog {
        &self.special_recipes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        demon::{DemonContent, DemonId, DemonMeta},
        race::Race,
        recipe::RecipeMeta,
        skill::{Skill, SkillCategory, SkillFlags, SkillId},
    };

    fn demon(id: u32) -> DemonMeta {
        DemonMeta {
            id: DemonId(id),
            race: Race::Fairy,
            base_level: 1,
            content: DemonContent::Base,
            compendium_price: 0,
            natural_skills: Vec::new(),
            innate_skill: SkillId(0),
        }
    }

    fn skill(id: u32) -> Skill {
        Skill {
            id: SkillId(id),
            category: SkillCategory::Physical,
            cost: 0,
            rank: 0,
            inheritable: true,
            flags: SkillFlags::default(),
        }
    }

    #[test]
    fn owns_meta_catalogs() {
        let demons = DemonCatalog::new(vec![demon(1), demon(2), demon(3)]);
        let skills = SkillCatalog::new(vec![skill(1), skill(2)]);
        let special_recipes = SpecialRecipeCatalog::new(vec![RecipeMeta {
            result: DemonId(3),
            materials: vec![DemonId(1), DemonId(2)],
            is_special: true,
        }]);

        let data = GameData::new(demons, skills, special_recipes);

        assert_eq!(data.demons().iter().len(), 3);
        assert!(data.skills().get(SkillId(2)).is_some());
        assert!(data.special_recipes().get(DemonId(3)).is_some());
    }
}
