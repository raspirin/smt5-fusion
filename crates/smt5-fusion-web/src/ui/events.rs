use wasm_bindgen::JsCast;

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

pub(super) fn event_checked(event: &web_sys::Event) -> bool {
    use wasm_bindgen::JsCast;
    event
        .target()
        .and_then(|target| target.dyn_into::<web_sys::HtmlInputElement>().ok())
        .is_some_and(|input| input.checked())
}
