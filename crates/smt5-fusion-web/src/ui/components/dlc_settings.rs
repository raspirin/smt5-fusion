use leptos::prelude::*;

use crate::{i18n::Message, protocol::DemonContent};

use super::super::{events::event_checked, state::Controller};

#[component]
pub(super) fn DlcSettings() -> impl IntoView {
    let controller = expect_context::<Controller>();
    let state = controller.state;
    let i18n = state.i18n;

    view! {
        <fieldset class="field-group">
            <legend>{move || i18n.text(Message::DlcSettings)}</legend>
            <label class="check-row">
                <input
                    type="checkbox"
                    prop:checked=move || state.konohana_sakuya_dlc.get()
                    on:change={
                        let controller = controller.clone();
                        move |event| controller.set_dlc(
                            DemonContent::KonohanaSakuyaDlc,
                            event_checked(&event),
                        )
                    }
                />
                <span>{move || i18n.content_name(DemonContent::KonohanaSakuyaDlc)}</span>
            </label>
            <label class="check-row">
                <input
                    type="checkbox"
                    prop:checked=move || state.dagda_dlc.get()
                    on:change=move |event| controller.set_dlc(
                        DemonContent::DagdaDlc,
                        event_checked(&event),
                    )
                />
                <span>{move || i18n.content_name(DemonContent::DagdaDlc)}</span>
            </label>
        </fieldset>
    }
}
