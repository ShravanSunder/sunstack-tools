use std::{
    future::Future,
    io::{self, Write},
};
use sunstack_decisions::CancellationToken;
use tokio::signal::unix::{Signal, SignalKind, signal};

pub struct ProcessSignals {
    interrupt: Signal,
    terminate: Signal,
}
impl ProcessSignals {
    pub fn install() -> io::Result<Self> {
        Ok(Self {
            interrupt: signal(SignalKind::interrupt())?,
            terminate: signal(SignalKind::terminate())?,
        })
    }
    pub async fn drive<TOutput>(
        &mut self,
        operation: impl Future<Output = TOutput>,
        cancellation: &CancellationToken,
    ) -> TOutput {
        tokio::pin!(operation);
        loop {
            tokio::select! {
                biased;
                _ = self.interrupt.recv() => observe_cancellation(cancellation),
                _ = self.terminate.recv() => observe_cancellation(cancellation),
                result = &mut operation => return result,
            }
        }
    }
}
fn observe_cancellation(cancellation: &CancellationToken) {
    if !cancellation.is_cancelled() {
        cancellation.cancel();
        operational_message(b"sunstack: cancellation observed\n");
    }
}
pub fn operational_message(message: &'static [u8]) {
    let mut stderr = io::stderr().lock();
    let _ = stderr.write_all(message);
    let _ = stderr.flush();
}
