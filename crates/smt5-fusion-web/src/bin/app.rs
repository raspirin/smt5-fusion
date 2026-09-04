#[cfg(target_arch = "wasm32")]
fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(smt5_fusion_web::ui::App);
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {}
