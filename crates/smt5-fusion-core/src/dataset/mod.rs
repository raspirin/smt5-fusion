pub mod demon_ids;
pub mod skill_ids;

mod demons;
mod skills;
mod special_recipes;

use crate::data::{
    demon_catalog::DemonCatalog, game_data::GameData, skill_catalog::SkillCatalog,
    special_recipe_catalog::SpecialRecipeCatalog,
};

pub fn game_data() -> GameData {
    GameData::new(
        DemonCatalog::new(demons::create()),
        SkillCatalog::new(skills::create()),
        SpecialRecipeCatalog::new(special_recipes::create()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        demon::{DemonContent, NaturalSkill, SkillAcquisition},
        race::{Element, Race},
        skill::{SkillCategory, SkillFlags},
    };

    #[test]
    fn loads_expected_catalog_sizes() {
        let data = game_data();

        assert_eq!(data.demons().iter().len(), 275);
        assert_eq!(data.skills().iter().len(), 763);
        assert_eq!(data.special_recipes().iter().len(), 62);
        assert_eq!(
            data.demons()
                .iter()
                .map(|demon| demon.natural_skills.len())
                .sum::<usize>(),
            1_575
        );
        assert_eq!(
            data.special_recipes()
                .iter()
                .map(|recipe| recipe.materials.len())
                .sum::<usize>(),
            193
        );
    }

    #[test]
    fn loads_representative_demon_facts() {
        let data = game_data();
        let abaddon = data.demons().get(demon_ids::ABADDON).unwrap();

        assert_eq!(abaddon.race, Race::Tyrant);
        assert_eq!(abaddon.base_level, 72);
        assert_eq!(abaddon.compendium_price, 30_558);
        assert_eq!(abaddon.natural_skills.len(), 7);
        assert_eq!(
            abaddon.natural_skills[0],
            NaturalSkill {
                skill: skill_ids::SEVERING_BITE,
                acquisition: SkillAcquisition::Initial { order: 1 },
            }
        );
        assert_eq!(
            abaddon.natural_skills[4],
            NaturalSkill {
                skill: skill_ids::SAFEGUARD,
                acquisition: SkillAcquisition::Level { level: 73 },
            }
        );
        assert_eq!(abaddon.innate_skill, skill_ids::CRIPPLING_BLOW);

        assert_eq!(
            data.demons()
                .get(demon_ids::KONOHANA_SAKUYA)
                .unwrap()
                .content,
            DemonContent::KonohanaSakuyaDlc
        );
        assert_eq!(
            data.demons().get(demon_ids::DAGDA).unwrap().content,
            DemonContent::DagdaDlc
        );
        assert_eq!(
            data.demons()
                .iter()
                .filter(|demon| demon.content != DemonContent::Base)
                .count(),
            2
        );

        for (element, id) in [
            (Element::Erthys, demon_ids::ERTHYS),
            (Element::Aeros, demon_ids::AEROS),
            (Element::Aquans, demon_ids::AQUANS),
            (Element::Flaemis, demon_ids::FLAEMIS),
        ] {
            assert_eq!(data.demons().element_id(element), Some(id));
        }
    }

    #[test]
    fn loads_representative_skill_flags() {
        let data = game_data();
        let agi = data.skills().get(skill_ids::AGI).unwrap();

        assert_eq!(agi.category, SkillCategory::Fire);
        assert_eq!(agi.cost, 10);
        assert_eq!(agi.rank, 1);
        assert!(agi.inheritable);
        assert_eq!(agi.flags, SkillFlags::default());

        let sakuya_sakura = data.skills().get(skill_ids::SAKUYA_SAKURA).unwrap();
        assert!(!sakuya_sakura.inheritable);
        assert!(sakuya_sakura.flags.unique);
        assert!(!sakuya_sakura.flags.magatsuhi);
        assert!(!sakuya_sakura.flags.item_only);

        let omagatoki_critical = data.skills().get(skill_ids::OMAGATOKI_CRITICAL).unwrap();
        assert!(!omagatoki_critical.inheritable);
        assert!(omagatoki_critical.flags.magatsuhi);
        assert!(omagatoki_critical.flags.item_only);

        let crippling_blow = data.skills().get(skill_ids::CRIPPLING_BLOW).unwrap();
        assert!(!crippling_blow.inheritable);
        assert_eq!(crippling_blow.category, SkillCategory::Innate);
        assert!(crippling_blow.flags.item_only);
    }

    #[test]
    fn inheritable_skills_are_normalized() {
        let data = game_data();
        let inheritable_skills = data
            .skills()
            .iter()
            .filter(|skill| skill.inheritable)
            .collect::<Vec<_>>();

        assert_eq!(inheritable_skills.len(), 257);
        assert!(inheritable_skills.iter().all(|skill| {
            !skill.flags.unique
                && !skill.flags.magatsuhi
                && !skill.flags.item_only
                && skill.category != SkillCategory::Innate
        }));
    }

    #[test]
    fn loads_two_three_and_four_material_special_recipes() {
        let data = game_data();

        assert_eq!(
            data.special_recipes()
                .get(demon_ids::SHIVA)
                .unwrap()
                .materials
                .as_slice(),
            &[demon_ids::BARONG, demon_ids::RANGDA]
        );
        assert_eq!(
            data.special_recipes()
                .get(demon_ids::SATAN)
                .unwrap()
                .materials
                .as_slice(),
            &[demon_ids::LUCIFER, demon_ids::SAMAEL, demon_ids::MASTEMA,]
        );
        assert_eq!(
            data.special_recipes()
                .get(demon_ids::ALICE)
                .unwrap()
                .materials
                .as_slice(),
            &[
                demon_ids::MUU_SHUWUU,
                demon_ids::POLTERGEIST,
                demon_ids::BUGS,
                demon_ids::JACK_O_LANTERN,
            ]
        );

        let mut recipe_counts_by_materials = [0; 5];
        for recipe in data.special_recipes().iter() {
            recipe_counts_by_materials[recipe.materials.len()] += 1;
        }
        assert_eq!(&recipe_counts_by_materials[2..=4], &[7, 41, 14]);
    }

    #[test]
    fn all_generated_references_resolve() {
        let data = game_data();

        for demon in data.demons().iter() {
            assert!(data.skills().get(demon.innate_skill).is_some());
            assert!(
                demon
                    .natural_skills
                    .iter()
                    .all(|natural| data.skills().get(natural.skill).is_some())
            );
        }

        for recipe in data.special_recipes().iter() {
            assert!(recipe.is_special);
            assert!(data.demons().get(recipe.result).is_some());
            assert!(
                recipe
                    .materials
                    .iter()
                    .all(|material| data.demons().get(*material).is_some())
            );
        }
    }

    #[test]
    fn construction_is_repeatable() {
        let first = game_data();
        let second = game_data();

        assert!(first.demons().iter().eq(second.demons().iter()));
        assert!(first.skills().iter().eq(second.skills().iter()));
        assert!(
            first
                .special_recipes()
                .iter()
                .eq(second.special_recipes().iter())
        );
    }

    #[test]
    fn regular_fusion_candidates_are_below_level_99() {
        let data = game_data();
        let level_99 = data
            .demons()
            .iter()
            .filter(|demon| demon.base_level == 99)
            .map(|demon| demon.id)
            .collect::<Vec<_>>();

        assert_eq!(level_99, vec![demon_ids::LUCIFER, demon_ids::SATAN]);
        assert!(
            level_99
                .iter()
                .all(|id| data.special_recipes().get(*id).is_some())
        );

        let regular_candidates = data
            .demons()
            .iter()
            .filter(|demon| data.special_recipes().get(demon.id).is_none())
            .collect::<Vec<_>>();
        assert!(regular_candidates.iter().all(|demon| demon.base_level < 99));

        let highest_regular = regular_candidates
            .into_iter()
            .max_by_key(|demon| demon.base_level)
            .unwrap();
        assert_eq!(highest_regular.id, demon_ids::METATRON);
        assert_eq!(highest_regular.base_level, 95);
    }
}
