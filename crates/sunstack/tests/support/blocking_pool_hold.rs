//! A test occupant of a runtime's only blocking thread.
//!
//! On a runtime built with `max_blocking_threads(1)`, blocking tasks run one at a time
//! in spawn order, so an occupant that has started proves every task spawned before it
//! has returned, and every task spawned after it waits until it is released.

/// Holds the runtime's only blocking thread until released; released on drop as well.
pub struct BlockingPoolHold {
    release: Option<std::sync::mpsc::Sender<()>>,
    task: Option<tokio::task::JoinHandle<()>>,
    started: Option<tokio::sync::oneshot::Receiver<()>>,
}

impl BlockingPoolHold {
    /// Queue an occupant behind every blocking task already spawned.
    pub fn queue() -> Self {
        let (release, released) = std::sync::mpsc::channel::<()>();
        let (running, started) = tokio::sync::oneshot::channel::<()>();
        let task = tokio::task::spawn_blocking(move || {
            running.send(()).unwrap();
            let _ = released.recv();
        });
        Self {
            release: Some(release),
            task: Some(task),
            started: Some(started),
        }
    }

    /// Wait until the occupant is running: every earlier blocking task has returned.
    pub async fn started(mut self) -> Self {
        self.started.take().unwrap().await.unwrap();
        self
    }

    /// Occupy the single blocking thread and wait until the occupant is running.
    pub async fn occupy() -> Self {
        Self::queue().started().await
    }

    pub async fn release(mut self) {
        drop(self.release.take());
        self.task.take().unwrap().await.unwrap();
    }
}

impl Drop for BlockingPoolHold {
    fn drop(&mut self) {
        // A failing assertion still frees the blocking thread so the runtime can shut down.
        drop(self.release.take());
    }
}

/// A current-thread runtime with one blocking thread.
pub fn one_blocking_thread_runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .max_blocking_threads(1)
        .enable_all()
        .build()
        .unwrap()
}
