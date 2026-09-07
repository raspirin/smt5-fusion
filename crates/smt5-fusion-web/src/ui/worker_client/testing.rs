use std::sync::{
    Mutex,
    atomic::{AtomicBool, Ordering},
};

use super::{Arc, WorkerRequest, WorkerResponse};

#[derive(Clone)]
pub(crate) struct TestWorker {
    requests: Arc<Mutex<Vec<WorkerRequest>>>,
    terminated: Arc<AtomicBool>,
    on_response: Arc<dyn Fn(WorkerResponse) + Send + Sync>,
    on_error: Arc<dyn Fn(String) + Send + Sync>,
}

impl TestWorker {
    pub(crate) fn take_requests(&self) -> Vec<WorkerRequest> {
        std::mem::take(&mut self.requests.lock().unwrap())
    }

    pub(crate) fn respond(&self, response: WorkerResponse) {
        (self.on_response)(response);
    }

    pub(crate) fn fail(&self, details: &str) {
        (self.on_error)(details.to_owned());
    }

    pub(crate) fn terminated(&self) -> bool {
        self.terminated.load(Ordering::Relaxed)
    }
}

pub struct WorkerClient {
    handle: TestWorker,
}

impl WorkerClient {
    pub fn new(
        on_response: Arc<dyn Fn(WorkerResponse) + Send + Sync>,
        on_error: Arc<dyn Fn(String) + Send + Sync>,
    ) -> Result<Self, String> {
        Ok(Self {
            handle: TestWorker {
                requests: Arc::new(Mutex::new(Vec::new())),
                terminated: Arc::new(AtomicBool::new(false)),
                on_response,
                on_error,
            },
        })
    }

    pub fn send(&self, request: &WorkerRequest) -> Result<(), String> {
        if self.handle.terminated() {
            return Err("worker terminated".to_owned());
        }
        self.handle.requests.lock().unwrap().push(request.clone());
        Ok(())
    }

    pub(crate) fn test_handle(&self) -> TestWorker {
        self.handle.clone()
    }
}

impl Drop for WorkerClient {
    fn drop(&mut self) {
        self.handle.terminated.store(true, Ordering::Relaxed);
    }
}
