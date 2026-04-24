# AGENTS.md

Contributor workflow and guardrails for `CorePolicyManager`, with emphasis on the Rust daemon (`CoreShift`).

## Architecture Map

- `rust/src/core`: pure state machine, reducers, scheduler, replay, validation, and invariants.
- `rust/src/high_level`: addon semantics, Android-facing policy mapping, identity, and capability rules.
- `rust/src/mid_level`: IPC framing and daemon boundary translation.
- `rust/src/low_level`: reactor, syscalls, spawn, drain, and IO primitives.
- `rust/src/runtime`: side effects, structured logging, Android/system services, and process execution.
- Android app code is a separate frontend. Rust daemon work must not casually change Android UI behavior.

## How To Work In This Repo

- Work file by file. Avoid broad rewrites across the whole tree in one pass.
- Prefer small scoped commits that compile and validate independently.
- Do not touch unrelated Android app code while making Rust daemon changes.
- Never assume docs are optional. If behavior changes, review `README.md`, `docs/*`, `AGENTS.md`, and nearby rustdoc in the same pass.
- Keep user-facing docs and developer docs aligned with actual runtime behavior.

Before each commit, ask:
`Did code behavior change? If yes, what docs changed with it?`

## Rust Runtime Guardrails

- No `println!`, `eprintln!`, or `dbg!` in production daemon paths.
- Structured daemon output must go through `LogLevel`, `LogEvent`, and the runtime log router.
- CLI help output in `rust/src/main.rs` is the allowed stdout exception for `help`, `--help`, and `-h`.
- IPC queue growth must be bounded before appending new frames. Overflow must be logged and the client must be dropped explicitly.
- Spawn argument validation must reject invalid exec context up front; do not silently drop malformed argv/env/cwd or rewrite commands to fallback binaries.
- Prefer recoverable errors over release-path panics in runtime-facing code.

## Required Validation Before Commit

Run from `rust/`:

```bash
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

If this environment reports `error: jobs may not be 0`, rerun with `-j 1`:

```bash
cargo fmt --check
cargo check -j 1
cargo test -j 1
cargo clippy -j 1 --all-targets --all-features -- -D warnings
```

Use additional target builds when the change affects Android packaging, spawn/runtime behavior, or ABI-sensitive code.

## Commit Standards

- Every commit must be production-quality and justify existing in history.
- No filler commits.
- No cosmetic churn without functional or documentation value.
- No meaningless rename-only commits.
- Prefer fewer stronger commits over stacked micro-commits on the same surface.

Commit message format:

```text
scope(area): concise professional summary

Intent: why this change exists.
Impact: what behavior or maintenance property changed.
Risk: note user-visible behavior changes, compatibility considerations, or “none” if the change is internal only.

Signed-off-by: <git user.name> <git user.email>
```

Examples:

- `rust(runtime): route daemon output through structured logging`
- `rust(core): replace stale-handle panics with checked state access`
- `rust(docs): align runtime behavior documentation with current daemon semantics`

## DCO Signoff Requirement

- Every commit must use `git commit --signoff`.
- If `user.name` or `user.email` is unset, stop and configure them before committing.
- Do not create unsigned follow-up commits.

## When To Update Docs

Update docs in the same logical commit when you change:

- CLI behavior or command semantics
- logging behavior or log locations
- IPC failure semantics
- spawn validation or process execution rules
- architecture boundaries or module responsibilities
- build, validation, or release commands

No placeholder docs. Keep docs concise, accurate, and operational.

## Safe Local Binary Use

- Use `coreshift --help` to verify the shell-facing CLI path without entering daemon mode.
- Treat daemon mode as a long-lived service process with structured log output, not a normal stdout tool.
- Prefer explicit control files under `/data/local/tmp/coreshift/control/` when validating runtime toggles on device.
- Check structured logs instead of expecting daemon diagnostics on stdout/stderr.
