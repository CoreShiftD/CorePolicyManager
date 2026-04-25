# Development Guide

This guide describes how to safely extend CoreShift while maintaining architectural integrity.

## Adding a New Command

1. Define the command in `rust/src/high_level/api.rs` within the `Command` enum (e.g., `Cmd`, `Dumpsys`).
2. Map its intent into an `ExecSpec` and `ExecPolicy` within the `map_to_exec` implementation.

## Adding a New Addon

1. Implement the `Addon` trait in `rust/src/high_level/addon.rs`.
2. Define its target execution limits and permissions via `CapabilityToken` in `high_level::capability`.
3. Wire the addon into the daemon startup sequence inside `rust/src/lib.rs` `run_daemon` block.

## Modifying Reducers and Actions safely

- **Rule**: Pure reducers must not cause side effects.
- If you need a new outcome (like logging or watching a stream), you must emit an `Action` from the reducer, map it to an `Effect` via the engine dispatcher, and write a system-level handler in `runtime/effects.rs` to fulfill the `Effect`.

## Commit Standards

Commits must be formatted professionally, describe the rationale clearly, and include a sign-off.
Example:
```bash
git commit --signoff -m "rust(core): add graceful shutdown action to scheduler"
```
