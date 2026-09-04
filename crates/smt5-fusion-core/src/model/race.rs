#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Element {
    Erthys,
    Aeros,
    Aquans,
    Flaemis,
}

impl Element {
    pub(crate) const COUNT: usize = 4;

    pub(crate) const fn index(self) -> usize {
        match self {
            Self::Erthys => 0,
            Self::Aeros => 1,
            Self::Aquans => 2,
            Self::Flaemis => 3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Race {
    Herald,
    Megami,
    Avian,
    Divine,
    Yoma,
    Vile,
    Raptor,
    Deity,
    Wargod,
    Avatar,
    Holy,
    Genma,
    Element(Element),
    Fairy,
    Beast,
    Jirae,
    Fiend,
    Jaki,
    Wilder,
    Fury,
    Lady,
    Dragon,
    Kishin,
    Kunitsu,
    Femme,
    Brute,
    Fallen,
    Night,
    Snake,
    Tyrant,
    Drake,
    Haunt,
    Foul,
    Enigma,
    Uma,
    Qadistu,
    Devil,
    Primal,
    Proto,
    Panagia,
    King,
    Human,
}

impl Race {
    pub(crate) const FUSION_COUNT: usize = 37;

    pub(crate) const fn fusion_index(self) -> Option<usize> {
        match self {
            Self::Herald => Some(0),
            Self::Megami => Some(1),
            Self::Avian => Some(2),
            Self::Divine => Some(3),
            Self::Yoma => Some(4),
            Self::Vile => Some(5),
            Self::Raptor => Some(6),
            Self::Deity => Some(7),
            Self::Wargod => Some(8),
            Self::Avatar => Some(9),
            Self::Holy => Some(10),
            Self::Genma => Some(11),
            Self::Fairy => Some(12),
            Self::Beast => Some(13),
            Self::Jirae => Some(14),
            Self::Fiend => Some(15),
            Self::Jaki => Some(16),
            Self::Wilder => Some(17),
            Self::Fury => Some(18),
            Self::Lady => Some(19),
            Self::Dragon => Some(20),
            Self::Kishin => Some(21),
            Self::Kunitsu => Some(22),
            Self::Femme => Some(23),
            Self::Brute => Some(24),
            Self::Fallen => Some(25),
            Self::Night => Some(26),
            Self::Snake => Some(27),
            Self::Tyrant => Some(28),
            Self::Drake => Some(29),
            Self::Haunt => Some(30),
            Self::Foul => Some(31),
            Self::Enigma => Some(32),
            Self::Uma => Some(33),
            Self::Qadistu => Some(34),
            Self::Devil => Some(35),
            Self::Primal => Some(36),
            _ => None,
        }
    }
}
