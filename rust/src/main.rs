use std::process::ExitCode;

fn main() -> ExitCode {
    coreshift_policy::cli::run_cli_from_env()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrapper_delegates_to_policy_cli() {
        let code = coreshift_policy::cli::run_cli_args(&Vec::new());
        assert_eq!(code, ExitCode::SUCCESS);
    }
}
