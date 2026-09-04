use leptos::prelude::*;

use crate::{
    i18n::text,
    protocol::{AcquisitionDto, OptionAcquisitionDto},
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum AcquisitionKind {
    Direct,
    Normal,
    Special,
}

impl AcquisitionKind {
    pub(super) fn from_route(acquisition: &AcquisitionDto) -> Self {
        match acquisition {
            AcquisitionDto::Direct { .. } => Self::Direct,
            AcquisitionDto::Fusion {
                is_special: true, ..
            } => Self::Special,
            AcquisitionDto::Fusion {
                is_special: false, ..
            } => Self::Normal,
        }
    }

    pub(super) fn from_option(acquisition: &OptionAcquisitionDto) -> Self {
        match acquisition {
            OptionAcquisitionDto::Direct { .. } => Self::Direct,
            OptionAcquisitionDto::Fusion {
                is_special: true, ..
            } => Self::Special,
            OptionAcquisitionDto::Fusion {
                is_special: false, ..
            } => Self::Normal,
        }
    }

    pub(super) fn route_card_class(self) -> &'static str {
        match self {
            Self::Direct => "route-node card direct-node",
            Self::Normal => "route-node card normal-node",
            Self::Special => "route-node card special-node",
        }
    }

    pub(super) fn option_card_class(self) -> &'static str {
        match self {
            Self::Direct => "picker-item option-card direct-option",
            Self::Normal => "picker-item option-card normal-option",
            Self::Special => "picker-item option-card special-option",
        }
    }

    fn label(self, upgraded: bool) -> &'static str {
        match (self, upgraded) {
            (Self::Direct, true) => text::DIRECT_AND_UPGRADE,
            (Self::Direct, false) => text::DIRECT,
            (Self::Normal, _) => text::NORMAL_FUSION,
            (Self::Special, _) => text::SPECIAL_FUSION,
        }
    }

    fn class(self) -> &'static str {
        match self {
            Self::Direct => "method-label direct",
            Self::Normal => "method-label normal",
            Self::Special => "method-label special",
        }
    }
}

#[component]
pub(super) fn MethodLabel(kind: AcquisitionKind, upgraded: bool) -> impl IntoView {
    view! { <span class=kind.class()>{kind.label(upgraded)}</span> }
}
