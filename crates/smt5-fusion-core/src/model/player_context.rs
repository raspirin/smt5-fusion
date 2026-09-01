use crate::{data::game_data::GameData, direct_recipe_index::DirectRecipeIndex};

use super::{demon::DemonId, recipe::RecipeMeta};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PlayerContext {
    konohana_sakuya_dlc: bool,
    dagda_dlc: bool,
    direct_recipes: Option<DirectRecipeIndex>,
}

impl PlayerContext {
    pub fn get_konohana_sakuya_dlc(&self) -> bool {
        self.konohana_sakuya_dlc
    }

    pub fn get_dagda_dlc(&self) -> bool {
        self.dagda_dlc
    }

    pub fn set_konohana_sakuya_dlc(&mut self, enabled: bool) {
        if self.konohana_sakuya_dlc != enabled {
            self.konohana_sakuya_dlc = enabled;
            self.direct_recipes = None;
        }
    }

    pub fn set_dagda_dlc(&mut self, enabled: bool) {
        if self.dagda_dlc != enabled {
            self.dagda_dlc = enabled;
            self.direct_recipes = None;
        }
    }

    pub fn prepare_direct_recipes(&mut self, game_data: &GameData) {
        if self.direct_recipes.is_none() {
            let direct_recipes = DirectRecipeIndex::build(game_data, self);
            self.direct_recipes = Some(direct_recipes);
        }
    }

    pub fn get_direct_recipes(&self, target: DemonId) -> Option<&[RecipeMeta]> {
        self.direct_recipes
            .as_ref()
            .expect("direct recipes must be prepared before querying")
            .get(target)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::{
        demon_catalog::DemonCatalog, skill_catalog::SkillCatalog,
        special_recipe_catalog::SpecialRecipeCatalog,
    };

    fn empty_game_data() -> GameData {
        GameData::new(
            DemonCatalog::new(Vec::new()),
            SkillCatalog::new(Vec::new()),
            SpecialRecipeCatalog::new(Vec::new()),
        )
    }

    #[test]
    fn default_disables_dlc() {
        let context = PlayerContext::default();

        assert!(!context.get_konohana_sakuya_dlc());
        assert!(!context.get_dagda_dlc());
    }

    #[test]
    fn setters_update_dlc_settings() {
        let mut context = PlayerContext::default();

        context.set_konohana_sakuya_dlc(true);
        context.set_dagda_dlc(true);

        assert!(context.get_konohana_sakuya_dlc());
        assert!(context.get_dagda_dlc());

        context.set_konohana_sakuya_dlc(false);
        context.set_dagda_dlc(false);

        assert!(!context.get_konohana_sakuya_dlc());
        assert!(!context.get_dagda_dlc());
    }

    #[test]
    #[should_panic(expected = "direct recipes must be prepared before querying")]
    fn direct_recipes_require_preparation() {
        PlayerContext::default().get_direct_recipes(DemonId(1));
    }

    #[test]
    fn direct_recipes_are_prepared_once_and_invalidated_by_changes() {
        let data = empty_game_data();
        let mut context = PlayerContext::default();

        assert!(context.direct_recipes.is_none());
        context.prepare_direct_recipes(&data);
        assert!(context.get_direct_recipes(DemonId(1)).is_none());
        let first_index = context.direct_recipes.as_ref().unwrap() as *const DirectRecipeIndex;

        context.set_dagda_dlc(false);
        context.prepare_direct_recipes(&data);
        assert!(context.get_direct_recipes(DemonId(1)).is_none());
        let reused_index = context.direct_recipes.as_ref().unwrap() as *const DirectRecipeIndex;
        assert_eq!(first_index, reused_index);

        context.set_dagda_dlc(true);
        assert!(context.direct_recipes.is_none());
        context.prepare_direct_recipes(&data);
        assert!(context.get_direct_recipes(DemonId(1)).is_none());
        assert!(context.direct_recipes.is_some());
    }
}
