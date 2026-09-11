use leptos::prelude::*;
use send_wrapper::SendWrapper;
use wasm_bindgen::{JsCast, JsValue, closure::Closure};

use crate::i18n::Message;

use super::super::{
    events::{active_element, restore_focus, trap_tab},
    state::Controller,
};

#[component]
pub(super) fn PickerDialog(
    heading_id: &'static str,
    title: Signal<String>,
    on_close: Callback<()>,
    return_focus_id: Option<String>,
    children: Children,
) -> impl IntoView {
    let i18n = expect_context::<Controller>().state.i18n;
    let return_focus = SendWrapper::new(active_element());
    let viewport = track_viewport();
    on_cleanup(move || {
        leptos::leptos_dom::helpers::queue_microtask(move || {
            if active_element().is_none_or(|element| element.tag_name() == "BODY") {
                let target = return_focus_id
                    .and_then(|id| web_sys::window()?.document()?.get_element_by_id(&id))
                    .and_then(|element| element.dyn_into().ok())
                    .or_else(|| return_focus.take());
                restore_focus(target);
            }
        });
    });
    view! {
        <div
            class="modal-backdrop"
            role="presentation"
            on:click=move |_| on_close.run(())
        >
            <div
                class="picker-viewport"
                style=move || viewport.get().map(Viewport::style)
            >
                <section
                    class="picker-panel"
                    role="dialog"
                    tabindex="-1"
                    aria-modal="true"
                    aria-labelledby=heading_id
                    on:click=move |event: web_sys::MouseEvent| event.stop_propagation()
                    on:keydown=move |event: web_sys::KeyboardEvent| {
                        if event.key() == "Escape" {
                            event.prevent_default();
                            event.stop_propagation();
                            on_close.run(());
                        } else {
                            trap_tab(&event);
                        }
                    }
                >
                    <div class="picker-heading">
                        <strong id=heading_id>{move || title.get()}</strong>
                        <button
                            class="icon-button"
                            type="button"
                            aria-label=move || i18n.text(Message::Close)
                            on:click=move |_| on_close.run(())
                        >
                            "×"
                        </button>
                    </div>
                    {children()}
                </section>
            </div>
        </div>
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Viewport {
    left: f64,
    top: f64,
    width: f64,
    height: f64,
}

impl Viewport {
    fn new(left: f64, top: f64, width: f64, height: f64) -> Option<Self> {
        ([left, top, width, height].into_iter().all(f64::is_finite) && width > 0.0 && height > 0.0)
            .then_some(Self {
                left: left.max(0.0),
                top: top.max(0.0),
                width,
                height,
            })
    }

    fn read(viewport: &web_sys::EventTarget) -> Option<Self> {
        let number = |name: &str| {
            js_sys::Reflect::get(viewport, &JsValue::from_str(name))
                .ok()?
                .as_f64()
        };
        Self::new(
            number("offsetLeft")?,
            number("offsetTop")?,
            number("width")?,
            number("height")?,
        )
    }

    fn style(self) -> String {
        format!(
            "left: {}px; top: {}px; width: {}px; height: {}px",
            self.left, self.top, self.width, self.height
        )
    }
}

fn track_viewport() -> RwSignal<Option<Viewport>> {
    let bounds = RwSignal::new(None);
    let viewport = web_sys::window()
        .and_then(|window| js_sys::Reflect::get(&window, &"visualViewport".into()).ok())
        .and_then(|viewport| viewport.dyn_into::<web_sys::EventTarget>().ok());
    if let Some(viewport) = viewport {
        bounds.set(Viewport::read(&viewport));
        let target = viewport.clone();
        let listener = Closure::<dyn FnMut(web_sys::Event)>::new(move |_| {
            bounds.try_set(Viewport::read(&target));
        });
        for event in ["resize", "scroll"] {
            let _ =
                viewport.add_event_listener_with_callback(event, listener.as_ref().unchecked_ref());
        }
        let subscription = SendWrapper::new((viewport, listener));
        on_cleanup(move || {
            let (viewport, listener) = &*subscription;
            for event in ["resize", "scroll"] {
                let _ = viewport
                    .remove_event_listener_with_callback(event, listener.as_ref().unchecked_ref());
            }
        });
    }
    bounds
}

#[cfg(test)]
mod tests {
    use super::Viewport;

    #[test]
    fn dialog_tracks_visible_bounds_when_the_keyboard_resizes_or_pans_the_viewport() {
        for (left, top, width, height) in [
            (0.0, 0.0, 390.0, 700.0),
            (0.0, 0.0, 390.0, 360.0),
            (0.0, 120.0, 390.0, 360.0),
            (80.0, 120.0, 195.0, 180.0),
        ] {
            let viewport = Viewport::new(left, top, width, height).unwrap();
            assert_eq!(
                viewport.style(),
                format!("left: {left}px; top: {top}px; width: {width}px; height: {height}px")
            );
        }
    }

    #[test]
    fn invalid_viewport_sizes_use_css_fallback_and_offsets_stay_nonnegative() {
        for (width, height) in [(0.0, 700.0), (390.0, -1.0), (f64::NAN, 700.0)] {
            assert!(Viewport::new(0.0, 0.0, width, height).is_none());
        }
        assert!(Viewport::new(f64::INFINITY, 0.0, 390.0, 700.0).is_none());
        assert_eq!(
            Viewport::new(-10.0, -20.0, 390.0, 700.0).unwrap(),
            Viewport::new(0.0, 0.0, 390.0, 700.0).unwrap()
        );
    }
}
