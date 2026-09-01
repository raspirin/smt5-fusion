use std::collections::BTreeMap;

use crate::{
    data::game_data::GameData,
    forward_fuse::{fuse, is_available},
    model::{demon::DemonId, player_context::PlayerContext, recipe::RecipeMeta},
};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct DirectRecipeIndex {
    recipes_by_result: BTreeMap<DemonId, Vec<RecipeMeta>>,
}

impl DirectRecipeIndex {
    pub(crate) fn build(game_data: &GameData, player_context: &PlayerContext) -> Self {
        let mut recipes_by_result = BTreeMap::<DemonId, Vec<RecipeMeta>>::new();

        for recipe in game_data.special_recipes().iter() {
            if is_available_special_recipe(game_data, player_context, recipe) {
                recipes_by_result
                    .entry(recipe.result)
                    .or_default()
                    .push(recipe.clone());
            }
        }

        let material_ids = game_data
            .demons()
            .iter()
            .map(|demon| demon.id)
            .collect::<Vec<_>>();

        for (left_index, &left) in material_ids.iter().enumerate() {
            for &right in &material_ids[left_index + 1..] {
                let Some(result) = fuse(game_data, player_context, left, right) else {
                    continue;
                };

                recipes_by_result
                    .entry(result)
                    .or_default()
                    .push(RecipeMeta {
                        result,
                        materials: vec![left, right],
                        is_special: false,
                    });
            }
        }

        Self { recipes_by_result }
    }

    pub(crate) fn get(&self, result: DemonId) -> Option<&[RecipeMeta]> {
        self.recipes_by_result.get(&result).map(Vec::as_slice)
    }
}

fn is_available_special_recipe(
    game_data: &GameData,
    player_context: &PlayerContext,
    recipe: &RecipeMeta,
) -> bool {
    game_data
        .demons()
        .get(recipe.result)
        .is_some_and(|result| is_available(result, player_context))
        && recipe.materials.iter().all(|material| {
            game_data
                .demons()
                .get(*material)
                .is_some_and(|material| is_available(material, player_context))
        })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::{
        data::{
            demon_catalog::DemonCatalog, skill_catalog::SkillCatalog,
            special_recipe_catalog::SpecialRecipeCatalog,
        },
        dataset::{self, demon_ids},
        model::{
            demon::{DemonContent, DemonMeta},
            race::Race,
            skill::SkillId,
        },
    };

    fn demon(id: u32, race: Race, base_level: u32) -> DemonMeta {
        DemonMeta {
            id: DemonId(id),
            race,
            base_level,
            content: DemonContent::Base,
            compendium_price: 0,
            natural_skills: Vec::new(),
            innate_skill: SkillId(0),
        }
    }

    fn special_recipe(result: u32, materials: &[u32]) -> RecipeMeta {
        RecipeMeta {
            result: DemonId(result),
            materials: materials.iter().copied().map(DemonId).collect(),
            is_special: true,
        }
    }

    fn game_data(demons: Vec<DemonMeta>, special_recipes: Vec<RecipeMeta>) -> GameData {
        GameData::new(
            DemonCatalog::new(demons),
            SkillCatalog::new(Vec::new()),
            SpecialRecipeCatalog::new(special_recipes),
        )
    }

    #[test]
    fn indexes_normal_recipes_in_material_order() {
        let data = game_data(
            vec![
                demon(5, Race::Genma, 11),
                demon(4, Race::Megami, 10),
                demon(2, Race::Herald, 10),
                demon(3, Race::Megami, 10),
                demon(1, Race::Herald, 10),
            ],
            Vec::new(),
        );
        let index = DirectRecipeIndex::build(&data, &PlayerContext::default());

        assert_eq!(
            index.get(DemonId(5)).unwrap(),
            [
                RecipeMeta {
                    result: DemonId(5),
                    materials: vec![DemonId(1), DemonId(3)],
                    is_special: false,
                },
                RecipeMeta {
                    result: DemonId(5),
                    materials: vec![DemonId(1), DemonId(4)],
                    is_special: false,
                },
                RecipeMeta {
                    result: DemonId(5),
                    materials: vec![DemonId(2), DemonId(3)],
                    is_special: false,
                },
                RecipeMeta {
                    result: DemonId(5),
                    materials: vec![DemonId(2), DemonId(4)],
                    is_special: false,
                },
            ]
        );
        assert!(index.get(DemonId(99)).is_none());
    }

    #[test]
    fn indexes_only_available_special_recipes() {
        let mut dagda_material = demon(4, Race::Herald, 10);
        dagda_material.content = DemonContent::DagdaDlc;
        let mut dagda_result = demon(8, Race::Genma, 10);
        dagda_result.content = DemonContent::DagdaDlc;
        let data = game_data(
            vec![
                demon(1, Race::Herald, 10),
                demon(2, Race::Megami, 10),
                demon(3, Race::Genma, 11),
                dagda_material,
                demon(5, Race::Genma, 12),
                dagda_result,
            ],
            vec![
                special_recipe(3, &[2, 1]),
                special_recipe(5, &[1, 4]),
                special_recipe(8, &[1, 2]),
            ],
        );
        let mut context = PlayerContext::default();

        let index = DirectRecipeIndex::build(&data, &context);
        assert_eq!(index.get(DemonId(3)).unwrap(), [special_recipe(3, &[2, 1])]);
        assert!(index.get(DemonId(5)).is_none());
        assert!(index.get(DemonId(8)).is_none());

        context.set_dagda_dlc(true);
        let index = DirectRecipeIndex::build(&data, &context);
        assert_eq!(index.get(DemonId(5)).unwrap(), [special_recipe(5, &[1, 4])]);
        assert_eq!(index.get(DemonId(8)).unwrap(), [special_recipe(8, &[1, 2])]);
    }

    fn player_context(konohana_sakuya_dlc: bool, dagda_dlc: bool) -> PlayerContext {
        let mut context = PlayerContext::default();
        context.set_konohana_sakuya_dlc(konohana_sakuya_dlc);
        context.set_dagda_dlc(dagda_dlc);
        context
    }

    #[test]
    fn formal_index_matches_every_successful_forward_pair() {
        let data = dataset::game_data();
        let material_ids = data
            .demons()
            .iter()
            .map(|demon| demon.id)
            .collect::<Vec<_>>();

        for (konohana_sakuya_dlc, dagda_dlc) in
            [(false, false), (true, false), (false, true), (true, true)]
        {
            let context = player_context(konohana_sakuya_dlc, dagda_dlc);
            let index = DirectRecipeIndex::build(&data, &context);
            assert_eq!(index, DirectRecipeIndex::build(&data, &context));

            let regular_recipes = index
                .recipes_by_result
                .values()
                .flatten()
                .filter(|recipe| !recipe.is_special)
                .collect::<Vec<_>>();
            let actual = regular_recipes
                .iter()
                .map(|recipe| {
                    let [left, right] = recipe.materials.as_slice() else {
                        panic!("regular recipes must have two materials");
                    };
                    assert!(left < right);
                    assert_eq!(fuse(&data, &context, *left, *right), Some(recipe.result));
                    (*left, *right, recipe.result)
                })
                .collect::<BTreeSet<_>>();
            assert_eq!(actual.len(), regular_recipes.len());

            let mut expected = BTreeSet::new();
            for (left_index, &left) in material_ids.iter().enumerate() {
                for &right in &material_ids[left_index + 1..] {
                    if let Some(result) = fuse(&data, &context, left, right) {
                        expected.insert((left, right, result));
                    }
                }
            }

            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn formal_index_filters_dlc_and_special_recipes() {
        let data = dataset::game_data();

        for (konohana_sakuya_dlc, dagda_dlc) in
            [(false, false), (true, false), (false, true), (true, true)]
        {
            let context = player_context(konohana_sakuya_dlc, dagda_dlc);
            let index = DirectRecipeIndex::build(&data, &context);

            for recipe in index.recipes_by_result.values().flatten() {
                let result = data.demons().get(recipe.result).unwrap();
                assert!(is_available(result, &context));
                assert!(recipe.materials.iter().all(|material| {
                    is_available(data.demons().get(*material).unwrap(), &context)
                }));
            }

            let indexed_specials = index
                .recipes_by_result
                .values()
                .flatten()
                .filter(|recipe| recipe.is_special);
            let expected_specials = data
                .special_recipes()
                .iter()
                .filter(|recipe| is_available_special_recipe(&data, &context, recipe));
            assert!(indexed_specials.eq(expected_specials));
        }

        let disabled = DirectRecipeIndex::build(&data, &PlayerContext::default());
        assert!(disabled.get(demon_ids::KONOHANA_SAKUYA).is_none());
        assert!(disabled.get(demon_ids::DAGDA).is_none());

        let konohana_sakuya = DirectRecipeIndex::build(&data, &player_context(true, false));
        assert!(konohana_sakuya.get(demon_ids::KONOHANA_SAKUYA).is_some());
        assert!(konohana_sakuya.get(demon_ids::DAGDA).is_none());

        let dagda = DirectRecipeIndex::build(&data, &player_context(false, true));
        assert!(dagda.get(demon_ids::KONOHANA_SAKUYA).is_none());
        assert!(dagda.get(demon_ids::DAGDA).is_some());
    }

    #[test]
    fn formal_index_covers_every_recipe_category() {
        let data = dataset::game_data();
        let context = player_context(true, true);
        let index = DirectRecipeIndex::build(&data, &context);
        let mut different_race_count = 0;
        let mut same_race_count = 0;
        let mut element_shift_count = 0;
        let mut special_material_counts = BTreeSet::new();

        for recipe in index.recipes_by_result.values().flatten() {
            if recipe.is_special {
                special_material_counts.insert(recipe.materials.len());
                continue;
            }

            let left = data.demons().get(recipe.materials[0]).unwrap();
            let right = data.demons().get(recipe.materials[1]).unwrap();
            match (left.race, right.race) {
                (Race::Element(_), _) | (_, Race::Element(_)) => element_shift_count += 1,
                (left_race, right_race) if left_race == right_race => same_race_count += 1,
                _ => different_race_count += 1,
            }
        }

        assert!(different_race_count > 0);
        assert!(same_race_count > 0);
        assert!(element_shift_count > 0);
        assert_eq!(special_material_counts, BTreeSet::from([2, 3, 4]));
    }

    #[test]
    fn formal_player_context_reuses_and_rebuilds_the_index() {
        let data = dataset::game_data();
        let mut context = PlayerContext::default();
        context.prepare_direct_recipes(&data);
        let target = data
            .demons()
            .iter()
            .map(|demon| demon.id)
            .find(|target| {
                context
                    .get_direct_recipes(*target)
                    .is_some_and(|recipes| !recipes.is_empty())
            })
            .unwrap();
        let initial_recipes = context.get_direct_recipes(target).unwrap().as_ptr();

        context.prepare_direct_recipes(&data);
        assert_eq!(
            initial_recipes,
            context.get_direct_recipes(target).unwrap().as_ptr()
        );
        context.set_dagda_dlc(false);
        assert_eq!(
            initial_recipes,
            context.get_direct_recipes(target).unwrap().as_ptr()
        );

        context.set_dagda_dlc(true);
        context.prepare_direct_recipes(&data);
        assert!(
            context
                .get_direct_recipes(demon_ids::DAGDA)
                .is_some_and(|recipes| !recipes.is_empty())
        );
    }
}
