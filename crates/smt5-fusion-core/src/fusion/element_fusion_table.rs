use crate::model::race::{Element, Race};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RankShift {
    Up,
    Down,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ElementFusionTable {
    rows: &'static [Option<[RankShift; Element::COUNT]>; Race::FUSION_COUNT],
}

impl ElementFusionTable {
    const fn new(rows: &'static [Option<[RankShift; Element::COUNT]>; Race::FUSION_COUNT]) -> Self {
        Self { rows }
    }

    pub(crate) fn get_offset(&self, race: Race, element: Element) -> Option<RankShift> {
        self.rows
            .get(race.fusion_index()?)?
            .as_ref()?
            .get(element.index())
            .copied()
    }
}

pub(crate) const ELEMENT_FUSION_TABLE: ElementFusionTable = ElementFusionTable::new(&ROWS);

const ROWS: [Option<[RankShift; Element::COUNT]>; Race::FUSION_COUNT] = [
    Some([
        RankShift::Down,
        RankShift::Down,
        RankShift::Down,
        RankShift::Up,
    ]),
    Some([
        RankShift::Down,
        RankShift::Down,
        RankShift::Up,
        RankShift::Down,
    ]),
    Some([
        RankShift::Down,
        RankShift::Up,
        RankShift::Down,
        RankShift::Down,
    ]),
    Some([
        RankShift::Down,
        RankShift::Down,
        RankShift::Up,
        RankShift::Up,
    ]),
    Some([
        RankShift::Down,
        RankShift::Up,
        RankShift::Up,
        RankShift::Down,
    ]),
    Some([
        RankShift::Down,
        RankShift::Down,
        RankShift::Up,
        RankShift::Down,
    ]),
    Some([
        RankShift::Down,
        RankShift::Up,
        RankShift::Down,
        RankShift::Up,
    ]),
    Some([
        RankShift::Down,
        RankShift::Down,
        RankShift::Down,
        RankShift::Up,
    ]),
    Some([
        RankShift::Up,
        RankShift::Down,
        RankShift::Down,
        RankShift::Down,
    ]),
    Some([
        RankShift::Down,
        RankShift::Down,
        RankShift::Up,
        RankShift::Down,
    ]),
    Some([
        RankShift::Down,
        RankShift::Down,
        RankShift::Down,
        RankShift::Up,
    ]),
    Some([
        RankShift::Down,
        RankShift::Down,
        RankShift::Up,
        RankShift::Down,
    ]),
    Some([
        RankShift::Up,
        RankShift::Down,
        RankShift::Up,
        RankShift::Down,
    ]),
    Some([
        RankShift::Down,
        RankShift::Up,
        RankShift::Down,
        RankShift::Up,
    ]),
    Some([
        RankShift::Up,
        RankShift::Up,
        RankShift::Down,
        RankShift::Down,
    ]),
    None,
    Some([
        RankShift::Up,
        RankShift::Down,
        RankShift::Down,
        RankShift::Up,
    ]),
    Some([
        RankShift::Down,
        RankShift::Down,
        RankShift::Up,
        RankShift::Up,
    ]),
    Some([
        RankShift::Down,
        RankShift::Up,
        RankShift::Down,
        RankShift::Down,
    ]),
    Some([
        RankShift::Up,
        RankShift::Down,
        RankShift::Down,
        RankShift::Down,
    ]),
    Some([
        RankShift::Down,
        RankShift::Up,
        RankShift::Down,
        RankShift::Down,
    ]),
    Some([
        RankShift::Up,
        RankShift::Down,
        RankShift::Down,
        RankShift::Down,
    ]),
    Some([
        RankShift::Down,
        RankShift::Down,
        RankShift::Down,
        RankShift::Up,
    ]),
    Some([RankShift::Up, RankShift::Down, RankShift::Up, RankShift::Up]),
    Some([RankShift::Up, RankShift::Down, RankShift::Up, RankShift::Up]),
    Some([
        RankShift::Down,
        RankShift::Up,
        RankShift::Down,
        RankShift::Up,
    ]),
    Some([
        RankShift::Down,
        RankShift::Up,
        RankShift::Down,
        RankShift::Down,
    ]),
    Some([
        RankShift::Down,
        RankShift::Down,
        RankShift::Up,
        RankShift::Up,
    ]),
    Some([
        RankShift::Down,
        RankShift::Down,
        RankShift::Down,
        RankShift::Up,
    ]),
    Some([
        RankShift::Up,
        RankShift::Down,
        RankShift::Down,
        RankShift::Up,
    ]),
    Some([
        RankShift::Down,
        RankShift::Up,
        RankShift::Down,
        RankShift::Down,
    ]),
    Some([
        RankShift::Down,
        RankShift::Down,
        RankShift::Up,
        RankShift::Down,
    ]),
    None,
    None,
    None,
    None,
    None,
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_has_thirty_one_supported_races() {
        assert_eq!(ROWS.iter().flatten().count(), 31);
    }

    #[test]
    fn lookup_uses_the_element_column_order() {
        assert_eq!(
            ELEMENT_FUSION_TABLE.get_offset(Race::Fairy, Element::Erthys),
            Some(RankShift::Up),
        );
        assert_eq!(
            ELEMENT_FUSION_TABLE.get_offset(Race::Fairy, Element::Aeros),
            Some(RankShift::Down),
        );
    }

    #[test]
    fn unsupported_races_have_no_shift() {
        assert_eq!(
            ELEMENT_FUSION_TABLE.get_offset(Race::Fiend, Element::Erthys),
            None,
        );
        assert_eq!(
            ELEMENT_FUSION_TABLE.get_offset(Race::Element(Element::Erthys), Element::Aeros),
            None,
        );
    }
}
