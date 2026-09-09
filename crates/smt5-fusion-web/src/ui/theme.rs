use leptos::prelude::*;

use crate::i18n::Message;

#[cfg(target_arch = "wasm32")]
use crate::build_info::STORAGE_KEYS;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum ThemePreference {
    #[default]
    Auto,
    Dark,
    Light,
}

impl ThemePreference {
    pub(super) const ALL: [Self; 3] = [Self::Auto, Self::Dark, Self::Light];

    pub(super) const fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Dark => "dark",
            Self::Light => "light",
        }
    }

    #[cfg(any(target_arch = "wasm32", test))]
    pub(super) fn from_str(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|theme| theme.as_str() == value)
    }

    pub(super) const fn message(self) -> Message {
        match self {
            Self::Auto => Message::ThemeAuto,
            Self::Dark => Message::ThemeDark,
            Self::Light => Message::ThemeLight,
        }
    }

    fn resolve(self, system_dark: bool) -> &'static str {
        match self {
            Self::Dark => "dark",
            Self::Light => "light",
            Self::Auto if system_dark => "dark",
            Self::Auto => "light",
        }
    }
}

#[cfg(any(target_arch = "wasm32", test))]
fn should_animate(
    previous: Option<&str>,
    next: &str,
    animate: bool,
    reduced_motion: bool,
    visible: bool,
) -> bool {
    animate
        && !reduced_motion
        && visible
        && previous.is_some_and(|previous| matches!(previous, "light" | "dark") && previous != next)
}

#[derive(Clone, Copy)]
pub(super) struct ThemeState {
    preference: RwSignal<ThemePreference>,
    system_dark: RwSignal<bool>,
    transition: StoredValue<browser::Transition>,
}

impl ThemeState {
    pub(super) fn new(preference: ThemePreference, system_dark: bool) -> Self {
        Self {
            preference: RwSignal::new(preference),
            system_dark: RwSignal::new(system_dark),
            transition: StoredValue::new(browser::Transition::default()),
        }
    }

    pub(super) fn load() -> Self {
        #[cfg(target_arch = "wasm32")]
        {
            browser::load()
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            Self::new(ThemePreference::default(), false)
        }
    }

    pub(super) fn preference(self) -> ThemePreference {
        self.preference.get()
    }

    pub(super) fn resolved(self) -> &'static str {
        self.preference
            .get_untracked()
            .resolve(self.system_dark.get_untracked())
    }

    pub(super) fn set_preference(self, preference: ThemePreference) {
        self.preference.set(preference);
        self.sync_document(true);
        browser::save(preference);
    }

    #[cfg(any(target_arch = "wasm32", test))]
    pub(super) fn set_system_dark(self, system_dark: bool) {
        self.system_dark.set(system_dark);
        if self.preference.get_untracked() == ThemePreference::Auto {
            self.sync_document(true);
        }
    }

    fn sync_document(self, animate: bool) {
        browser::apply(
            self.preference.get_untracked(),
            self.resolved(),
            animate,
            self.transition,
        );
    }
}

#[cfg(target_arch = "wasm32")]
mod browser {
    use std::time::Duration;

    use leptos::leptos_dom::helpers::{TimeoutHandle, set_timeout_with_handle};
    use send_wrapper::SendWrapper;
    use wasm_bindgen::{JsCast, closure::Closure};

    use super::*;

    #[derive(Default)]
    pub(super) struct Transition {
        timeout: Option<TimeoutHandle>,
    }

    impl Transition {
        fn cancel_timer(&mut self) {
            if let Some(timeout) = self.timeout.take() {
                timeout.clear();
            }
        }
    }

    impl Drop for Transition {
        fn drop(&mut self) {
            self.cancel_timer();
            if let Some(root) = web_sys::window()
                .and_then(|window| window.document())
                .and_then(|document| document.document_element())
            {
                let _ = root.remove_attribute("data-theme-transition");
            }
        }
    }

    pub(super) fn load() -> ThemeState {
        let window = web_sys::window();
        let preference = window
            .as_ref()
            .and_then(|window| window.document())
            .and_then(|document| document.document_element())
            .and_then(|root| root.get_attribute("data-theme-preference"))
            .and_then(|value| ThemePreference::from_str(&value))
            .unwrap_or_default();
        let media = window.and_then(|window| {
            window
                .match_media("(prefers-color-scheme: dark)")
                .ok()
                .flatten()
        });
        let state = ThemeState::new(
            preference,
            media.as_ref().is_some_and(|media| media.matches()),
        );
        state.sync_document(false);
        if let Some(media) = media {
            let listener = Closure::<dyn FnMut(web_sys::MediaQueryListEvent)>::new(
                move |event: web_sys::MediaQueryListEvent| {
                    state.set_system_dark(event.matches());
                },
            );
            if media
                .add_event_listener_with_callback("change", listener.as_ref().unchecked_ref())
                .is_ok()
            {
                let subscription = SendWrapper::new((media, listener));
                on_cleanup(move || {
                    let (media, listener) = &*subscription;
                    let _ = media.remove_event_listener_with_callback(
                        "change",
                        listener.as_ref().unchecked_ref(),
                    );
                });
            }
        }
        state
    }

    pub(super) fn apply(
        preference: ThemePreference,
        scheme: &str,
        animate: bool,
        transition: StoredValue<Transition>,
    ) {
        let Some(window) = web_sys::window() else {
            return;
        };
        let Some(document) = window.document() else {
            return;
        };
        if let Some(root) = document.document_element() {
            let reduced_motion = window
                .match_media("(prefers-reduced-motion: reduce)")
                .ok()
                .flatten()
                .is_some_and(|media| media.matches());
            let visible = !document.hidden();
            let previous = root.get_attribute("data-theme");
            if should_animate(
                previous.as_deref(),
                scheme,
                animate,
                reduced_motion,
                visible,
            ) {
                transition.update_value(|transition| transition.cancel_timer());
                let _ = root.set_attribute("data-theme-transition", "");
                let _ = root.get_bounding_client_rect();
                let finished_root = root.clone();
                let timeout = set_timeout_with_handle(
                    move || {
                        if transition
                            .try_update_value(|transition| transition.timeout = None)
                            .is_some()
                        {
                            let _ = finished_root.remove_attribute("data-theme-transition");
                        }
                    },
                    Duration::from_millis(350),
                );
                match timeout {
                    Ok(timeout) => {
                        transition.update_value(|transition| transition.timeout = Some(timeout))
                    }
                    Err(_) => {
                        let _ = root.remove_attribute("data-theme-transition");
                    }
                }
            } else if !animate || reduced_motion || !visible {
                transition.update_value(|transition| transition.cancel_timer());
                let _ = root.remove_attribute("data-theme-transition");
            }
            let _ = root.set_attribute("data-theme", scheme);
            let _ = root.set_attribute("data-theme-preference", preference.as_str());
        }
        for theme in ["light", "dark"] {
            if let Ok(Some(meta)) =
                document.query_selector(&format!("meta[name='theme-color'][data-theme='{theme}']"))
            {
                let _ =
                    meta.set_attribute("media", if theme == scheme { "all" } else { "not all" });
            }
        }
    }

    pub(super) fn save(preference: ThemePreference) {
        if let Some(storage) =
            web_sys::window().and_then(|window| window.local_storage().ok().flatten())
        {
            let _ = storage.set_item(STORAGE_KEYS.theme, preference.as_str());
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod browser {
    use super::{StoredValue, ThemePreference};

    #[derive(Default)]
    pub(super) struct Transition {}

    pub(super) fn apply(
        _preference: ThemePreference,
        _scheme: &str,
        _animate: bool,
        _transition: StoredValue<Transition>,
    ) {
    }

    pub(super) fn save(_preference: ThemePreference) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_visible_color_changes_animate_when_motion_is_allowed() {
        for (previous, next) in [("light", "dark"), ("dark", "light")] {
            assert!(should_animate(Some(previous), next, true, false, true));
            assert!(!should_animate(Some(previous), next, false, false, true));
            assert!(!should_animate(Some(previous), next, true, true, true));
            assert!(!should_animate(Some(previous), next, true, false, false));
            assert!(!should_animate(Some(next), next, true, false, true));
            assert!(!should_animate(None, next, true, false, true));
            assert!(!should_animate(Some("invalid"), next, true, false, true));
        }
    }

    #[test]
    fn preference_values_round_trip_without_localization() {
        assert_eq!(ThemePreference::default(), ThemePreference::Auto);
        for preference in ThemePreference::ALL {
            assert_eq!(
                ThemePreference::from_str(preference.as_str()),
                Some(preference)
            );
        }
        for invalid in ["", "system", "Dark", "true", "深色", "null"] {
            assert_eq!(ThemePreference::from_str(invalid), None);
        }
    }

    #[test]
    fn automatic_theme_tracks_the_system_without_changing_the_preference() {
        Owner::new().with(|| {
            let theme = ThemeState::load();
            assert_eq!(theme.resolved(), "light");
            for dark in [true, false, true] {
                theme.set_system_dark(dark);
                assert_eq!(theme.resolved(), if dark { "dark" } else { "light" });
                assert_eq!(untrack(|| theme.preference()), ThemePreference::Auto);
            }
        });
    }

    #[test]
    fn explicit_preferences_override_the_system_and_auto_uses_its_latest_state() {
        Owner::new().with(|| {
            let theme = ThemeState::new(ThemePreference::Auto, false);
            for preference in [ThemePreference::Dark, ThemePreference::Light] {
                theme.set_preference(preference);
                for dark in [true, false, true] {
                    theme.set_system_dark(dark);
                    assert_eq!(theme.resolved(), preference.as_str());
                    assert_eq!(untrack(|| theme.preference()), preference);
                }
                theme.set_preference(ThemePreference::Auto);
                assert_eq!(theme.resolved(), "dark");
                theme.set_system_dark(false);
                assert_eq!(theme.resolved(), "light");
            }
        });
    }
}
