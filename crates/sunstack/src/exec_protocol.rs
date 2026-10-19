use crate::process_signals::{ProcessSignals, operational_message};
use crate::protocol_boundary::ProgramResponse;
use crate::{operation_execution, protocol_boundary};
use std::{
    io::{self, Write},
    process::ExitCode,
};
use sunstack_decisions::CancellationToken;

pub async fn run() -> ExitCode {
    let mut signals = match ProcessSignals::install() {
        Ok(signals) => signals,
        Err(_) => {
            operational_message(b"sunstack-rule: signal setup failed\n");
            return ExitCode::from(3);
        }
    };
    operational_message(b"sunstack: ready\n");
    let cancellation = CancellationToken::new();
    let request = signals
        .drive(protocol_boundary::read_frame(), &cancellation)
        .await
        .and_then(|bytes| protocol_boundary::decode_request(&bytes));
    let (response, exit) = match request {
        Ok(input) => {
            // Drive to owned settlement even after a signal. Dropping this
            // future would discard the libraries' cancellation progress.
            let result = signals
                .drive(
                    operation_execution::execute_operation(input, &cancellation),
                    &cancellation,
                )
                .await;
            let exit = result.exit_code();
            (ProgramResponse::operation_result(result), exit)
        }
        Err(code) => (ProgramResponse::protocol_error(code), 2),
    };
    let mut stdout = io::stdout().lock();
    if serde_json::to_writer(&mut stdout, &response).is_err()
        || stdout.write_all(b"\n").is_err()
        || stdout.flush().is_err()
    {
        operational_message(b"sunstack-rule: output unavailable\n");
        return ExitCode::from(3);
    }
    ExitCode::from(exit)
}
