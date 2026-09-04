#[cfg(target_arch = "wasm32")]
fn main() {
    smt5_fusion_web::service::run_worker();
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {}
