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

- `rust/src/core`
  Pure state machine, reducers, scheduler, replay, validation, invariants.
  Internal split:
  - `core/types.rs` for actions, events, effects, handles, and routing metadata
  - `core/state.rs` for execution-state models and views
  - `core/engine.rs` for dispatcher and reducer wiring

- `rust/src/high_level`
  Android-facing policy semantics, identity, capabilities, feature mapping.

- `rust/src/mid_level`
  IPC framing, daemon boundary translation, request/response transport.

- `rust/src/low_level`
  Reactor, syscalls, spawn, drain, IO primitives, OS interaction.

- `rust/src/runtime`
  Side effects, structured logging, services, process execution, orchestration.

- Android app code is a separate frontend. Rust daemon work must not casually alter Android UI behavior.

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
- Keep logging efficient and rate-limited when noisy.

---

## Validation Required Before Commit

Run from `rust/`:

```bash
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
