use std::process::ExitCode;

fn main() -> ExitCode {
    coreshift_policy::cli::run_cli_from_env()
}
