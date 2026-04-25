# AGENTS.md

Contributor workflow and autonomous agent guardrails for `CorePolicyManager`, with emphasis on the Rust daemon (`CoreShift`).

---

## Mission

Continuously improve the Rust daemon for:

1. correctness
2. stability
3. security
4. battery / performance
5. maintainability
6. documentation accuracy

Do not optimize for commit count. Optimize for durable engineering quality.

---

## Architecture Map

- **`rust/src/core`**: Pure state machine, reducers, scheduler, replay, validation, invariants.
  - Internal split: `core/types.rs`, `core/state.rs`, `core/engine.rs`
- **`rust/src/high_level`**: Android-facing policy semantics, identity, capabilities, feature mapping.
  - Addons own **policy/decision state only**: enabled flags, caches, skip reasons, warmup results.
  - Addons must **not** perform filesystem probes, daemon context lookups, or JSON serialization.
  - Addons expose typed snapshots (e.g. `PreloadSnapshot`) via trait methods; the runtime assembles reports.
  - Typed IPC request/response structs (`DaemonStatusReport`, `PreloadSnapshot`, `WatchedPathStatus`) live in `high_level/api.rs`.
- **`rust/src/mid_level`**: IPC framing, daemon boundary translation, request/response transport.
  - Frames bytes and dispatches commands. Must not know addon internals beyond command/response types.
  - Receives opaque serialized payloads from the runtime; does not assemble or interpret status reports.
- **`rust/src/low_level`**: **The ONLY layer allowed direct OS/platform-facing API access.** Reactor, syscalls, spawn, drain, IO primitives.
  - Path existence checks must use `low_level::sys::path_exists()`, not `std::path::Path::exists()`.
- **`rust/src/runtime`**: Side effects, structured logging, services, process execution, orchestration.
  - **Owns status assembly**: `runtime::status::assemble_daemon_status()` is the single place that combines addon snapshots, live filesystem probes, and daemon context into a `DaemonStatusReport`.
  - Calls `low_level::sys::path_exists()` for all control-file and device-path probes.
- **`rust/src/main.rs`**: Thin CLI only. Parses commands, sends typed IPC requests, deserializes typed responses, pretty-prints. No protocol logic beyond using shared constants.

Android app code is a separate frontend. Rust daemon work must not casually alter Android UI behavior.

---

## Documentation Map

- **User-Facing Guides**: `README.md`, `docs/quickstart.md`, `docs/daemon-usage.md`
- **Internal Rust Documentation**: `rust/README.md` and the `rust/docs/` directory (`architecture.md`, `runtime.md`, `ipc.md`, `build.md`, `testing.md`, `android.md`, `logging.md`).

---

## Primary Working Rules

- Prefer small scoped improvements that validate independently.
- Multi-file edits are allowed when required for one coherent improvement.
- Avoid broad rewrites unless explicitly justified.
- Do not touch unrelated Android UI code during Rust work.
- Keep the repository buildable after every commit.
- Prefer fewer strong commits over many weak commits.

Before each commit ask:

`Did behavior change? If yes, what docs changed too?`

---

## Autonomous Agent Operating Rules

When operating without human supervision:

- Do not stop after one failed attempt.
- Investigate failures and retry with a narrower fix.
- Do not repeat the same failed strategy.
- If no useful change was made, pivot to another subsystem.
- Never do 3 consecutive no-change passes on the same target.
- Leave clear artifacts/logs describing what happened.

If blocked:

1. inspect errors
2. identify root cause
3. make smallest safe fix
4. rerun validation
5. revert only failing changes if needed

Always end in a clean, buildable state.

---

## Decision Priority Order

When choosing work autonomously:

1. correctness bugs
2. crash / panic risks
3. data corruption risks
4. security issues
5. battery drain / hot polling
6. memory / FD leaks
7. maintainability debt
8. test gaps
9. documentation drift
10. cosmetic cleanup

---

## Minimum Commit Value

Do not create a commit unless at least one is true:

- fixes a bug
- removes panic risk
- improves reliability
- reduces complexity
- improves module boundaries
- adds useful tests
- improves measurable performance
- removes dead code
- improves documentation accuracy
- resolves build or lint regressions

Do not commit filler.

---

## If Unsure What To Do

Audit one subsystem and improve it:

- core state transitions
- timeout scheduling
- replay integrity
- IPC framing / backpressure
- process lifecycle
- spawn validation
- logging consistency
- panic paths
- battery wakeups
- Android integration boundaries
- docs accuracy

Then make one justified improvement.

---

## Layer Responsibility Rules

These rules were established after a status-reporting refactor revealed role drift.
Violating them creates hidden coupling that makes the codebase harder to test and audit.

**Addon (`high_level/addons/`):**
- Own policy/decision state: enabled flags, caches, skip reasons, warmup results, foreground tracking.
- Expose state via typed snapshot methods (e.g. `status_snapshot() -> PreloadSnapshot`).
- Must NOT call `std::path::Path::exists()`, `std::fs::*`, or any OS probe directly.
- Must NOT serialize to JSON or produce wire-format strings.
- Must NOT know socket paths, daemon mode, or control-file paths.

**`low_level/sys`:**
- Owns all direct OS/platform-facing checks: `path_exists()`, metadata, inotify registration.
- `path_exists(path: &str) -> bool` is the canonical helper; use it everywhere instead of `Path::exists()`.

**`runtime/status.rs`:**
- Single place for assembling `DaemonStatusReport`.
- Calls `low_level::sys::path_exists()` for filesystem probes.
- Calls `addon.preload_snapshot()` (via trait) for policy state.
- Merges daemon context (mode, socket path) passed in by the caller.

**`mid_level/ipc.rs`:**
- Frames bytes, dispatches commands, enforces backpressure.
- Receives opaque JSON strings from the runtime; does not parse or construct status reports.
- Must not import addon types beyond what `high_level/api.rs` defines.

**`high_level/api.rs`:**
- Defines stable typed request/response structs: `Command`, `DaemonStatusReport`, `PreloadSnapshot`, `WatchedPathStatus`.
- These are the canonical wire types; all layers reference them.

**`main.rs`:**
- Thin CLI: parse command, send IPC request, deserialize typed response, pretty-print.
- Must not duplicate protocol framing logic.
- Uses `paths::SOCKET_PATH` and `api::DaemonStatusReport` constants/types.

---

## Rust Runtime Guardrails

- No `println!`, `eprintln!`, or `dbg!` in production daemon paths.
- Structured output must go through `LogLevel`, `LogEvent`, and runtime logging.
- CLI help in `rust/src/main.rs` may print to stdout for:
  - `help`
  - `--help`
  - `-h`

- Prefer recoverable errors over release-path panics.
- Do not silently discard malformed exec argv/env/cwd.
- IPC queue growth must be bounded before append.
- IPC framing changes must preserve partial-frame handling on nonblocking sockets.
- Overflow must be logged and client state handled explicitly.
- Long-lived loops must avoid unnecessary wake polling.
- Fixed tick loops must sleep until the next useful deadline instead of bypassing the scheduler cadence.

---

## Performance / Battery Rules

Because this targets Android systems:

- Avoid busy loops.
- Sleep until real deadlines/events where possible.
- Avoid repeated heap allocations in hot paths.
- Prefer reusable buffers.
- Audit memory growth in long-lived services.
- Close file descriptors promptly.
- Keep logging efficient and rate-limited when noisy. Idle metrics should be rate-limited heavily unless trace/debug overrides are present.
- Control files (like `enable_preload`) remain available for manual toggles, even if CLI modes auto-enable features.

---

## Validation Required Before Commit

Run from `rust/`:

```bash
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```
