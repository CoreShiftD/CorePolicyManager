# Testing & Validation

CoreShift requires a strictly validated build before patches are submitted.

## Standard Cargo Validation

Because the environment and build agents can be severely resource-constrained, jobs must occasionally be restricted to `-j 1` to prevent `error: jobs may not be 0` or OOM kills.

Run the following from the `rust/` directory:
```bash
cargo fmt --check
cargo check -j 1
cargo test -j 1
cargo clippy -j 1 --all-targets --all-features -- -D warnings
```

## Record and Replay

Because `core` is entirely pure and deterministic, runtime debugging on mobile devices is minimized. You can capture state mutations to a trace file and replay them locally on your host machine.

1. **Capture**: Run the daemon on the device using `coreshift record <filename>`.
2. **Transfer**: Pull the trace file via `adb pull`.
3. **Replay**: Replay locally with `coreshift replay <filename>`.

This offline replay rebuilding guarantees zero-flakiness debugging of historical edge cases without the original Android host environment.
