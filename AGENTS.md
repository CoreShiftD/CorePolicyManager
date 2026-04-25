# Agent Guidelines for CoreShift Policy

This document defines the constraints and conventions for AI agents and contributors working on the **CoreShift Policy** daemon.

## Core Identity
- **Product Name**: CoreShift Policy
- **Executable Name**: `corepolicy`
- **Daemon Name**: CoreShift Policy Daemon

Contributors must use the `corepolicy` executable name in all new CLI documentation, examples, and scripts.

## CLI and Runtime Model
- **Autonomous Operation**: The daemon is designed to be autonomous. It observes system state and decides on actions locally. The Android application should not need to command it continuously.
- **Compact Flags**: Use compact, single-letter flags for features (e.g., `-p` for preload).
- **No IPC for Status**: Basic status reporting must not depend on IPC. The `corepolicy status` command reads from `/data/local/tmp/coreshift/status.json` directly.
- **Stable Substrate**: Keep the `low_level` module stable. It acts as the trusted OS boundary.
- **Implementation Status**: Only document and implement flags for features that are actually available. Future flags should be marked as **reserved** or **proposed**, not available.

## Implementation Priorities
1. **Correctness**: Ensure syscalls and resource ownership are handled safely in `low_level`.
2. **Minimalism**: Avoid complex IPC or heavy dependencies unless explicitly required.
3. **Transparency**: Use structured logging and the `status.json` file for observability.
