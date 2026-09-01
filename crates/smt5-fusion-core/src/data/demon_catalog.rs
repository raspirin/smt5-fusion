use std::collections::BTreeMap;

use crate::model::{
    demon::{DemonId, DemonMeta},
    race::{Element, Race},
};

#[derive(Debug)]
pub struct DemonCatalog {
    demons: Vec<DemonMeta>,
    ids_by_race: BTreeMap<Race, Vec<DemonId>>,
}

impl DemonCatalog {
    pub(crate) fn new(mut demons: Vec<DemonMeta>) -> Self {
        demons.sort_unstable_by_key(|demon| demon.id);

        let mut race_entries = BTreeMap::<Race, Vec<(u32, DemonId)>>::new();

        for demon in &demons {
            race_entries
                .entry(demon.race)
                .or_default()
                .push((demon.base_level, demon.id));
        }

        let ids_by_race = race_entries
            .into_iter()
            .map(|(race, mut entries)| {
                entries.sort_unstable();
                let ids = entries.into_iter().map(|(_, id)| id).collect::<Vec<_>>();
                (race, ids)
            })
            .collect();

        Self {
            demons,
            ids_by_race,
        }
    }

    pub fn get(&self, id: DemonId) -> Option<&DemonMeta> {
        let index = self
            .demons
            .binary_search_by_key(&id, |demon| demon.id)
            .ok()?;
        self.demons.get(index)
    }

    pub fn iter(&self) -> impl ExactSizeIterator<Item = &DemonMeta> {
        self.demons.iter()
    }

    pub(crate) fn ids_by_race(&self, race: Race) -> &[DemonId] {
        self.ids_by_race.get(&race).map_or(&[], Vec::as_slice)
    }

    pub(crate) fn element_id(&self, element: Element) -> Option<DemonId> {
        self.ids_by_race(Race::Element(element)).first().copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{demon::DemonContent, skill::SkillId};

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

    #[test]
    fn indexes_demons_by_id_and_race() {
        let catalog = DemonCatalog::new(vec![
            demon(3, Race::Fairy, 20),
            demon(1, Race::Fairy, 5),
            demon(2, Race::Fairy, 20),
            demon(4, Race::Divine, 10),
        ]);

        assert_eq!(catalog.get(DemonId(3)).unwrap().base_level, 20);
        assert_eq!(catalog.get(DemonId(9)), None);
        assert_eq!(
            catalog.ids_by_race(Race::Fairy),
            &[DemonId(1), DemonId(2), DemonId(3)]
        );
        assert_eq!(catalog.ids_by_race(Race::Yoma), &[]);
        assert_eq!(
            catalog.iter().map(|demon| demon.id).collect::<Vec<_>>(),
            vec![DemonId(1), DemonId(2), DemonId(3), DemonId(4)]
        );
    }

    #[test]
    fn indexes_elements() {
        let catalog = DemonCatalog::new(vec![demon(1, Race::Element(Element::Aeros), 12)]);

        assert_eq!(catalog.element_id(Element::Aeros), Some(DemonId(1)));
        assert_eq!(catalog.element_id(Element::Erthys), None);
    }
}
