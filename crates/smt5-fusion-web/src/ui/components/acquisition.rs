use leptos::prelude::*;

use crate::{
    i18n::Message,
    protocol::{AcquisitionDto, OptionAcquisitionDto},
};

use super::super::state::Controller;

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

    fn label(self, upgraded: bool) -> Message {
        match (self, upgraded) {
            (Self::Direct, true) => Message::DirectAndUpgrade,
            (Self::Direct, false) => Message::Direct,
            (Self::Normal, true) => Message::NormalFusionAndUpgrade,
            (Self::Normal, false) => Message::NormalFusion,
            (Self::Special, _) => Message::SpecialFusion,
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
    let i18n = expect_context::<Controller>().state.i18n;
    view! { <span class=kind.class()>{move || i18n.text(kind.label(upgraded))}</span> }
}

#[cfg(test)]
mod tests {
    use super::{AcquisitionKind, Message};

    #[test]
    fn normal_fusion_labels_include_leveling_only_when_needed() {
        assert_eq!(AcquisitionKind::Normal.label(false), Message::NormalFusion);
        assert_eq!(
            AcquisitionKind::Normal.label(true),
            Message::NormalFusionAndUpgrade
        );
        assert_eq!(AcquisitionKind::Direct.label(false), Message::Direct);
        assert_eq!(
            AcquisitionKind::Direct.label(true),
            Message::DirectAndUpgrade
        );
        assert_eq!(
            AcquisitionKind::Special.label(false),
            Message::SpecialFusion
        );
        assert_eq!(AcquisitionKind::Special.label(true), Message::SpecialFusion);
    }
}
