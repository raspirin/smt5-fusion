use wasm_bindgen::JsCast;

pub(super) const TOUCH_OR_NO_HOVER_QUERY: &str = "(any-pointer: coarse), (hover: none)";

pub(super) fn focus_picker_on_open(input: &web_sys::HtmlInputElement) {
    let touch_or_no_hover = web_sys::window()
        .and_then(|window| window.match_media(TOUCH_OR_NO_HOVER_QUERY).ok().flatten())
        .map(|media| media.matches());
    let target = if allows_text_autofocus(touch_or_no_hover) {
        Some(input.clone().unchecked_into::<web_sys::HtmlElement>())
    } else {
        input
            .closest("[role='dialog']")
            .ok()
            .flatten()
            .and_then(|panel| panel.dyn_into::<web_sys::HtmlElement>().ok())
    };
    restore_focus(target);
}

fn allows_text_autofocus(touch_or_no_hover: Option<bool>) -> bool {
    touch_or_no_hover == Some(false)
}

pub(super) fn preserve_picker_focus(event: web_sys::MouseEvent) {
    if event.button() == 0 {
        event.prevent_default();
    }
}

pub(super) fn focus_current_target(event: &web_sys::MouseEvent) {
    restore_focus(
        event
            .current_target()
            .and_then(|target| target.dyn_into().ok()),
    );
}

pub(super) fn focus_moved_outside(event: &web_sys::FocusEvent) -> bool {
    let Some(current) = event
        .current_target()
        .and_then(|target| target.dyn_into::<web_sys::Node>().ok())
    else {
        return true;
    };
    event
        .related_target()
        .and_then(|target| target.dyn_into::<web_sys::Node>().ok())
        .is_none_or(|next| !current.contains(Some(&next)))
}

pub(super) fn active_element() -> Option<web_sys::HtmlElement> {
    web_sys::window()?
        .document()?
        .active_element()?
        .dyn_into()
        .ok()
}

pub(super) fn restore_focus(target: Option<web_sys::HtmlElement>) {
    if let Some(target) = target
        && target.is_connected()
        && !target.matches(":disabled, [inert] *").unwrap_or(true)
    {
        let _ = target.focus();
    }
}

pub(super) fn trap_tab(event: &web_sys::KeyboardEvent) {
    if event.key() != "Tab" {
        return;
    }
    let Some(container) = event
        .current_target()
        .and_then(|target| target.dyn_into::<web_sys::Element>().ok())
    else {
        return;
    };
    let Ok(nodes) = container.query_selector_all(
        "button:not(:disabled), input:not(:disabled), select:not(:disabled), [href], [tabindex]:not([tabindex='-1'])",
    ) else {
        return;
    };
    let elements = (0..nodes.length())
        .filter_map(|index| nodes.item(index)?.dyn_into::<web_sys::HtmlElement>().ok())
        .collect::<Vec<_>>();
    let active = active_element();
    let index = elements
        .iter()
        .position(|element| Some(element) == active.as_ref());
    if let Some(index) = tab_wrap_index(elements.len(), index, event.shift_key()) {
        event.prevent_default();
        let _ = elements[index].focus();
    } else if elements.is_empty() {
        event.prevent_default();
    }
}

fn tab_wrap_index(count: usize, active: Option<usize>, backwards: bool) -> Option<usize> {
    if count == 0 {
        return None;
    }
    match (active, backwards) {
        (None | Some(0), true) => Some(count - 1),
        (None, false) => Some(0),
        (Some(index), false) if index + 1 >= count => Some(0),
        _ => None,
    }
}

pub(super) fn event_checked(event: &web_sys::Event) -> bool {
    event
        .target()
        .and_then(|target| target.dyn_into::<web_sys::HtmlInputElement>().ok())
        .is_some_and(|input| input.checked())
}

#[cfg(test)]
mod tests {
    use super::{allows_text_autofocus, tab_wrap_index};

    #[test]
    fn automatic_text_focus_requires_a_confirmed_non_touch_hover_environment() {
        assert!(allows_text_autofocus(Some(false)));
        assert!(!allows_text_autofocus(Some(true)));
        assert!(!allows_text_autofocus(None));
    }

    #[test]
    fn pickers_use_the_shared_focus_policy_and_have_focusable_dialog_containers() {
        for source in [
            include_str!("components/skill_picker.rs"),
            include_str!("components/source_picker.rs"),
        ] {
            assert!(source.contains("focus_picker_on_open(&input)"));
            assert!(source.contains("<PickerDialog"));
            assert!(!source.contains("input.focus()"));
            assert!(!source.contains("autofocus"));
        }
        let dialog = include_str!("components/picker_dialog.rs");
        assert!(dialog.contains("role=\"dialog\""));
        assert!(dialog.contains("tabindex=\"-1\""));
    }

    #[test]
    fn modal_tab_navigation_wraps_without_leaving_the_dialog() {
        assert_eq!(tab_wrap_index(3, Some(2), false), Some(0));
        assert_eq!(tab_wrap_index(3, Some(0), true), Some(2));
        assert_eq!(tab_wrap_index(3, Some(1), false), None);
        assert_eq!(tab_wrap_index(3, Some(1), true), None);
        assert_eq!(tab_wrap_index(3, None, false), Some(0));
        assert_eq!(tab_wrap_index(3, None, true), Some(2));
        assert_eq!(tab_wrap_index(1, Some(0), false), Some(0));
        assert_eq!(tab_wrap_index(0, None, false), None);
    }
}
