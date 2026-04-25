# Agent Guidelines for CoreShift

This document provides instructions for AI agents working on this codebase.

## Stable Substrate: rust/src/low_level

The `rust/src/low_level` directory contains mature OS-boundary code (reactor, inotify, syscalls, process primitives).

**Mandate:**
- Treat `low_level` as a stable substrate.
- Do NOT rewrite or broad-refactor components in this directory.
- Prefer making changes in the `runtime` or `high_level` layers.
- Wrap awkward `low_level` APIs in runtime adapters rather than modifying the internals.

**Modification Criteria:**
Modify `low_level` ONLY for:
1. Correctness bugs.
2. Android compatibility issues.
3. Safety issues.
4. Measurable performance bottlenecks.
5. Missing OS primitives needed by higher layers.
