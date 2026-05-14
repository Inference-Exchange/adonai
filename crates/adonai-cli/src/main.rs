use std::process::ExitCode;

use adonai_cli::{CliError, run_cli};

#[tokio::main]
async fn main() -> ExitCode {
    match run_cli(std::env::args().skip(1)).await {
        Ok(output) => {
            println!("{output}");
            ExitCode::SUCCESS
        }
        Err(CliError::Usage(message)) => {
            eprintln!("{message}");
            ExitCode::from(2)
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
