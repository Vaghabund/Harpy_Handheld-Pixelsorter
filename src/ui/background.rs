use eframe::egui;
use std::sync::mpsc::{self, Receiver, TryRecvError};

/// A job running on its own thread whose result is collected by the UI thread.
///
/// The UI polls it once per frame; the worker requests a repaint when it
/// finishes so the result is picked up even if the UI is otherwise idle.
/// Dropping the task discards its result (the thread still runs to completion).
pub struct BackgroundTask<T> {
    receiver: Receiver<T>,
}

impl<T: Send + 'static> BackgroundTask<T> {
    pub fn spawn(ctx: &egui::Context, name: &str, job: impl FnOnce() -> T + Send + 'static) -> Self {
        let (sender, receiver) = mpsc::channel();
        let ctx = ctx.clone();
        let spawned = std::thread::Builder::new().name(name.to_string()).spawn(move || {
            let _ = sender.send(job());
            ctx.request_repaint();
        });
        if let Err(e) = spawned {
            log::error!("Failed to start background task {}: {}", name, e);
        }
        Self { receiver }
    }
}

impl<T> BackgroundTask<T> {
    /// Returns the result once the job has finished.
    /// `Some(None)` means the job ended without a result (its thread panicked or never started).
    pub fn poll(&self) -> Option<Option<T>> {
        match self.receiver.try_recv() {
            Ok(result) => Some(Some(result)),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => Some(None),
        }
    }
}

/// Poll an optional task; when it has finished, clear the slot and return its outcome
pub fn take_finished<T>(slot: &mut Option<BackgroundTask<T>>) -> Option<Option<T>> {
    let outcome = slot.as_ref()?.poll()?;
    *slot = None;
    Some(outcome)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    fn wait_for<T>(slot: &mut Option<BackgroundTask<T>>) -> Option<T> {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if let Some(outcome) = take_finished(slot) {
                return outcome;
            }
            assert!(Instant::now() < deadline, "background task timed out");
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    #[test]
    fn returns_result_and_clears_slot() {
        let ctx = egui::Context::default();
        let mut slot = Some(BackgroundTask::spawn(&ctx, "test", || 6 * 7));
        assert_eq!(wait_for(&mut slot), Some(42));
        assert!(slot.is_none());
        assert!(take_finished(&mut slot).is_none());
    }

    #[test]
    fn reports_job_that_ended_without_result() {
        let ctx = egui::Context::default();
        let mut slot = Some(BackgroundTask::spawn(&ctx, "test-panic", || -> i32 { panic!("job failed") }));
        assert_eq!(wait_for(&mut slot), None);
    }
}
