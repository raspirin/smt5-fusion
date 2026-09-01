use std::cmp::Ordering;

use crate::{
    data::game_data::GameData,
    fusion::{
        element_fusion_table::{ELEMENT_FUSION_TABLE, RankShift},
        fusion_table::FUSION_TABLE,
    },
    model::{
        demon::{DemonContent, DemonId, DemonMeta},
        player_context::PlayerContext,
        race::{Element, Race},
    },
};

pub fn fuse(
    game_data: &GameData,
    player_context: &PlayerContext,
    left: DemonId,
    right: DemonId,
) -> Option<DemonId> {
    let left = game_data.demons().get(left)?;
    let right = game_data.demons().get(right)?;

    if left.id == right.id
        || !is_available(left, player_context)
        || !is_available(right, player_context)
    {
        return None;
    }

    match (left.race, right.race) {
        (Race::Element(_), Race::Element(_)) => None,
        (Race::Element(element), _) => fuse_with_element(game_data, player_context, element, right),
        (_, Race::Element(element)) => fuse_with_element(game_data, player_context, element, left),
        (left_race, right_race) if left_race == right_race => fuse_same_race(game_data, left_race),
        _ => fuse_different_races(game_data, player_context, left, right),
    }
}

pub(crate) fn is_available(demon: &DemonMeta, player_context: &PlayerContext) -> bool {
    match demon.content {
        DemonContent::Base => true,
        DemonContent::KonohanaSakuyaDlc => player_context.get_konohana_sakuya_dlc(),
        DemonContent::DagdaDlc => player_context.get_dagda_dlc(),
    }
}

fn get_candidates<'a>(
    game_data: &'a GameData,
    player_context: &'a PlayerContext,
    race: Race,
) -> impl DoubleEndedIterator<Item = &'a DemonMeta> + 'a {
    game_data
        .demons()
        .ids_by_race(race)
        .iter()
        .filter_map(move |id| {
            let demon = game_data.demons().get(*id)?;
            is_available(demon, player_context).then_some(demon)
        })
}

fn get_candidates_without_special<'a>(
    game_data: &'a GameData,
    player_context: &'a PlayerContext,
    race: Race,
) -> impl DoubleEndedIterator<Item = &'a DemonMeta> + 'a {
    get_candidates(game_data, player_context, race)
        .filter(|demon| game_data.special_recipes().get(demon.id).is_none())
}

fn fuse_same_race(game_data: &GameData, race: Race) -> Option<DemonId> {
    let Race::Element(element) = FUSION_TABLE.result(race, race)? else {
        return None;
    };

    game_data.demons().element_id(element)
}

fn fuse_with_element(
    game_data: &GameData,
    player_context: &PlayerContext,
    element: Element,
    material: &DemonMeta,
) -> Option<DemonId> {
    let shift = ELEMENT_FUSION_TABLE.get_offset(material.race, element)?;
    let is_special_material = game_data.special_recipes().get(material.id).is_some();
    let mut candidates = get_candidates_without_special(game_data, player_context, material.race);

    match shift {
        RankShift::Up => candidates
            .find(|candidate| compare_fusion_rank(candidate, material, is_special_material).is_gt())
            .map(|candidate| candidate.id),
        RankShift::Down => candidates
            .rev()
            .find(|candidate| compare_fusion_rank(candidate, material, is_special_material).is_lt())
            .map(|candidate| candidate.id),
    }
}

fn compare_fusion_rank(
    candidate: &DemonMeta,
    material: &DemonMeta,
    is_special_material: bool,
) -> Ordering {
    match candidate.base_level.cmp(&material.base_level) {
        Ordering::Equal if !is_special_material => candidate.id.cmp(&material.id),
        ordering => ordering,
    }
}

fn fuse_different_races(
    game_data: &GameData,
    player_context: &PlayerContext,
    left: &DemonMeta,
    right: &DemonMeta,
) -> Option<DemonId> {
    let result_race = FUSION_TABLE.result(left.race, right.race)?;
    let minimum_doubled_level = left.base_level + right.base_level + 2;
    get_candidates_without_special(game_data, player_context, result_race)
        .find(|candidate| 2 * candidate.base_level >= minimum_doubled_level)
        .or_else(|| {
            // 目标等级高于该种族所有候选时，游戏规则回退到最高可用位阶。
            get_candidates_without_special(game_data, player_context, result_race).next_back()
        })
        .map(|result| result.id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        data::{
            demon_catalog::DemonCatalog, skill_catalog::SkillCatalog,
            special_recipe_catalog::SpecialRecipeCatalog,
        },
        model::{recipe::RecipeMeta, skill::SkillId},
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

    fn game_data(demons: Vec<DemonMeta>, special_results: &[u32]) -> GameData {
        GameData::new(
            DemonCatalog::new(demons),
            SkillCatalog::new(Vec::new()),
            SpecialRecipeCatalog::new(
                special_results
                    .iter()
                    .map(|result| RecipeMeta {
                        result: DemonId(*result),
                        materials: vec![DemonId(1), DemonId(2)],
                        is_special: true,
                    })
                    .collect(),
            ),
        )
    }

    #[test]
    fn rejects_missing_duplicate_and_unavailable_materials() {
        let mut dagda = demon(5, Race::Herald, 10);
        dagda.content = DemonContent::DagdaDlc;
        let data = game_data(
            vec![
                demon(1, Race::Herald, 10),
                demon(2, Race::Megami, 10),
                demon(3, Race::Genma, 10),
                dagda,
            ],
            &[],
        );
        let mut context = PlayerContext::default();

        assert_eq!(fuse(&data, &context, DemonId(99), DemonId(2)), None);
        assert_eq!(fuse(&data, &context, DemonId(1), DemonId(1)), None);
        assert_eq!(fuse(&data, &context, DemonId(5), DemonId(2)), None);

        context.set_dagda_dlc(true);
        assert_eq!(
            fuse(&data, &context, DemonId(5), DemonId(2)),
            Some(DemonId(3)),
        );
    }

    #[test]
    fn different_race_fusion_uses_level_threshold_and_is_symmetric() {
        let data = game_data(
            vec![
                demon(1, Race::Herald, 10),
                demon(2, Race::Megami, 20),
                demon(3, Race::Genma, 15),
                demon(4, Race::Genma, 16),
                demon(5, Race::Genma, 30),
            ],
            &[],
        );
        let context = PlayerContext::default();

        assert_eq!(
            fuse(&data, &context, DemonId(1), DemonId(2)),
            Some(DemonId(4)),
        );
        assert_eq!(
            fuse(&data, &context, DemonId(2), DemonId(1)),
            Some(DemonId(4)),
        );
    }

    #[test]
    fn different_race_fusion_skips_special_and_unavailable_candidates() {
        let mut dlc_candidate = demon(4, Race::Genma, 20);
        dlc_candidate.content = DemonContent::KonohanaSakuyaDlc;
        let data = game_data(
            vec![
                demon(1, Race::Herald, 10),
                demon(2, Race::Megami, 20),
                demon(3, Race::Genma, 16),
                dlc_candidate,
                demon(5, Race::Genma, 25),
            ],
            &[3],
        );
        let mut context = PlayerContext::default();

        assert_eq!(
            fuse(&data, &context, DemonId(1), DemonId(2)),
            Some(DemonId(5)),
        );

        context.set_konohana_sakuya_dlc(true);
        assert_eq!(
            fuse(&data, &context, DemonId(1), DemonId(2)),
            Some(DemonId(4)),
        );
    }

    #[test]
    fn different_race_fusion_falls_back_to_the_highest_rank() {
        let data = game_data(
            vec![
                demon(1, Race::Herald, 80),
                demon(2, Race::Megami, 80),
                demon(3, Race::Genma, 20),
                demon(4, Race::Genma, 40),
            ],
            &[],
        );

        assert_eq!(
            fuse(&data, &PlayerContext::default(), DemonId(1), DemonId(2),),
            Some(DemonId(4)),
        );
    }

    #[test]
    fn same_race_fusion_returns_the_table_element() {
        let data = game_data(
            vec![
                demon(1, Race::Fairy, 5),
                demon(2, Race::Fairy, 10),
                demon(3, Race::Element(Element::Aeros), 10),
            ],
            &[],
        );
        let context = PlayerContext::default();

        assert_eq!(
            fuse(&data, &context, DemonId(1), DemonId(2)),
            Some(DemonId(3)),
        );
        assert_eq!(
            fuse(&data, &context, DemonId(2), DemonId(1)),
            Some(DemonId(3)),
        );
    }

    #[test]
    fn element_fusion_shifts_regular_ranks_and_skips_unavailable_candidates() {
        let mut dlc_candidate = demon(4, Race::Fairy, 20);
        dlc_candidate.content = DemonContent::KonohanaSakuyaDlc;
        let data = game_data(
            vec![
                demon(1, Race::Fairy, 5),
                demon(2, Race::Fairy, 10),
                dlc_candidate,
                demon(5, Race::Fairy, 30),
                demon(6, Race::Element(Element::Erthys), 10),
                demon(7, Race::Element(Element::Aeros), 10),
            ],
            &[],
        );
        let mut context = PlayerContext::default();

        assert_eq!(
            fuse(&data, &context, DemonId(6), DemonId(2)),
            Some(DemonId(5)),
        );
        assert_eq!(
            fuse(&data, &context, DemonId(2), DemonId(7)),
            Some(DemonId(1)),
        );
        assert_eq!(fuse(&data, &context, DemonId(6), DemonId(5)), None);
        assert_eq!(fuse(&data, &context, DemonId(7), DemonId(1)), None);

        context.set_konohana_sakuya_dlc(true);
        assert_eq!(
            fuse(&data, &context, DemonId(6), DemonId(2)),
            Some(DemonId(4)),
        );
    }

    #[test]
    fn element_fusion_positions_special_materials_by_level() {
        let data = game_data(
            vec![
                demon(1, Race::Fairy, 5),
                demon(2, Race::Fairy, 10),
                demon(3, Race::Fairy, 20),
                demon(4, Race::Fairy, 15),
                demon(5, Race::Element(Element::Erthys), 10),
                demon(6, Race::Element(Element::Aeros), 10),
            ],
            &[4],
        );
        let context = PlayerContext::default();

        assert_eq!(
            fuse(&data, &context, DemonId(5), DemonId(4)),
            Some(DemonId(3)),
        );
        assert_eq!(
            fuse(&data, &context, DemonId(6), DemonId(4)),
            Some(DemonId(2)),
        );
    }

    #[test]
    fn unsupported_element_and_race_combinations_have_no_result() {
        let data = game_data(
            vec![
                demon(1, Race::Element(Element::Erthys), 10),
                demon(2, Race::Element(Element::Aeros), 10),
                demon(3, Race::Fiend, 10),
                demon(4, Race::Fiend, 20),
                demon(5, Race::Herald, 10),
                demon(6, Race::Vile, 10),
            ],
            &[],
        );
        let context = PlayerContext::default();

        assert_eq!(fuse(&data, &context, DemonId(1), DemonId(2)), None);
        assert_eq!(fuse(&data, &context, DemonId(1), DemonId(3)), None);
        assert_eq!(fuse(&data, &context, DemonId(3), DemonId(4)), None);
        assert_eq!(fuse(&data, &context, DemonId(5), DemonId(6)), None);
    }
}
