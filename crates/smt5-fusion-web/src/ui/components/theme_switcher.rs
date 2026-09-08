use leptos::prelude::*;

use crate::i18n::Message;

use super::super::{
    events::event_checked,
    state::Controller,
    theme::{ThemePreference, ThemeState},
};

#[component]
pub(crate) fn ThemeSwitcher() -> impl IntoView {
    let i18n = expect_context::<Controller>().state.i18n;
    let theme = expect_context::<ThemeState>();

    view! {
        <fieldset class="theme-switch" aria-label=move || i18n.text(Message::ThemeSelector)>
            {ThemePreference::ALL.into_iter().map(|preference| {
                view! {
                    <label class="theme-option">
                        <input
                            type="radio"
                            name="color-theme"
                            value=preference.as_str()
                            prop:checked=move || theme.preference() == preference
                            on:change=move |event| {
                                if event_checked(&event) {
                                    theme.set_preference(preference);
                                }
                            }
                        />
                        <span>{move || i18n.text(preference.message())}</span>
                    </label>
                }
            }).collect_view()}
        </fieldset>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::{I18n, Locale};

    #[test]
    fn theme_choices_are_localized_in_the_expected_order() {
        Owner::new().with(|| {
            let i18n = I18n::new(Locale::ZhCn);
            for (locale, label, choices) in [
                (Locale::ZhCn, "外观", ["自动", "深色", "浅色"]),
                (Locale::ZhTw, "外觀", ["自動", "深色", "淺色"]),
                (Locale::EnUs, "Appearance", ["Auto", "Dark", "Light"]),
                (Locale::JaJp, "外観", ["自動", "ダーク", "ライト"]),
            ] {
                i18n.set_locale(locale);
                assert_eq!(i18n.text_untracked(Message::ThemeSelector), label);
                assert_eq!(
                    ThemePreference::ALL
                        .map(|preference| i18n.text_untracked(preference.message())),
                    choices.map(str::to_owned)
                );
            }
        });
    }
}
