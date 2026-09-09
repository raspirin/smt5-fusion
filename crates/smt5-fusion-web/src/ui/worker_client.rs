use std::sync::Arc;

use crate::protocol::{WorkerRequest, WorkerResponse};

#[cfg(target_arch = "wasm32")]
pub struct WorkerClient {
    worker: web_sys::Worker,
    _on_message: wasm_bindgen::closure::Closure<dyn FnMut(web_sys::MessageEvent)>,
    _on_error: wasm_bindgen::closure::Closure<dyn FnMut(web_sys::ErrorEvent)>,
}

#[cfg(target_arch = "wasm32")]
impl WorkerClient {
    pub fn new(
        on_response: Arc<dyn Fn(WorkerResponse) + Send + Sync>,
        on_error: Arc<dyn Fn(String) + Send + Sync>,
    ) -> Result<Self, String> {
        use wasm_bindgen::JsCast;
        use web_sys::{WorkerOptions, WorkerType};

        let document = web_sys::window()
            .and_then(|window| window.document())
            .ok_or("Missing document for Worker startup")?;
        let url = document
            .query_selector("meta[name='smt5-worker-url']")
            .map_err(js_error)?
            .and_then(|meta| meta.get_attribute("content"))
            .filter(|url| !url.trim().is_empty())
            .ok_or("Missing Worker URL in startup HTML")?;
        let options = WorkerOptions::new();
        options.set_type(WorkerType::Module);
        let worker = web_sys::Worker::new_with_options(&url, &options).map_err(js_error)?;

        let response_handler = Arc::clone(&on_response);
        let response_error_handler = Arc::clone(&on_error);
        let on_message = wasm_bindgen::closure::Closure::<dyn FnMut(web_sys::MessageEvent)>::new(
            move |event: web_sys::MessageEvent| match serde_wasm_bindgen::from_value::<WorkerResponse>(
                event.data(),
            ) {
                Ok(response) => response_handler(response),
                Err(error) => response_error_handler(error.to_string()),
            },
        );
        worker.set_onmessage(Some(on_message.as_ref().unchecked_ref()));

        let on_error = wasm_bindgen::closure::Closure::<dyn FnMut(web_sys::ErrorEvent)>::new(
            move |event: web_sys::ErrorEvent| {
                event.prevent_default();
                on_error(event.message());
            },
        );
        worker.set_onerror(Some(on_error.as_ref().unchecked_ref()));

        Ok(Self {
            worker,
            _on_message: on_message,
            _on_error: on_error,
        })
    }

    pub fn send(&self, request: &WorkerRequest) -> Result<(), String> {
        let value = serde_wasm_bindgen::to_value(request).map_err(|error| error.to_string())?;
        self.worker.post_message(&value).map_err(js_error)
    }
}

#[cfg(target_arch = "wasm32")]
impl Drop for WorkerClient {
    fn drop(&mut self) {
        self.worker.set_onmessage(None);
        self.worker.set_onerror(None);
        self.worker.terminate();
    }
}

#[cfg(target_arch = "wasm32")]
fn js_error(value: wasm_bindgen::JsValue) -> String {
    value.as_string().unwrap_or_else(|| format!("{value:?}"))
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod testing;
#[cfg(all(test, not(target_arch = "wasm32")))]
pub(super) use testing::TestWorker;
#[cfg(all(test, not(target_arch = "wasm32")))]
pub use testing::WorkerClient;

#[cfg(all(not(target_arch = "wasm32"), not(test)))]
pub struct WorkerClient;

#[cfg(all(not(target_arch = "wasm32"), not(test)))]
impl WorkerClient {
    pub fn new(
        _on_response: Arc<dyn Fn(WorkerResponse) + Send + Sync>,
        _on_error: Arc<dyn Fn(String) + Send + Sync>,
    ) -> Result<Self, String> {
        Err("Web Worker is only available on wasm32".to_owned())
    }

    pub fn send(&self, _request: &WorkerRequest) -> Result<(), String> {
        Err("Web Worker is only available on wasm32".to_owned())
    }
}
