use std::{
    sync::mpsc::{self, Receiver},
    thread::{self, JoinHandle},
};
pub(super) struct Task {
    pub receiver: Receiver<Result<Vec<String>, String>>,
    worker: Option<JoinHandle<()>>,
}
impl Task {
    pub fn start() -> Self {
        let (sender, receiver) = mpsc::sync_channel(1);
        let worker = thread::spawn(move || {
            let result = player_platform::fonts::enumerate();
            let _ = sender.send(result);
        });
        Self {
            receiver,
            worker: Some(worker),
        }
    }
}
impl Drop for Task {
    fn drop(&mut self) {
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
