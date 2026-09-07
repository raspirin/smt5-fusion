use wasm_bindgen::JsCast;

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
    use super::tab_wrap_index;

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
