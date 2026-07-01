use std::process::ExitCode;

fn main() -> ExitCode {
    match std::env::args().nth(1).as_deref() {
        Some("uhp") | None => match philanthus::run_uhp() {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("philanthus uhp error: {error}");
                ExitCode::FAILURE
            }
        },
        Some(other) => {
            eprintln!("unknown command '{other}'; usage: philanthus [uhp]");
            ExitCode::FAILURE
        }
    }
}
