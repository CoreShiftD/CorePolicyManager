# Agent Guidelines for CoreShift Policy

This document defines the constraints and conventions for AI agents and contributors working on the **CoreShift Policy** daemon.

## Core Identity
- **Product Name**: CoreShift Policy
- **Executable Name**: `corepolicy`
- **Daemon Name**: CoreShift Policy Daemon

Contributors must use the `corepolicy` executable name in all new CLI documentation, examples, and scripts.

## Current Project Reality
- **low_level Module**: Stable substrate. It acts as the trusted OS boundary and is currently the most complete part of the Rust codebase.
- **Runtime Daemon**: NOT implemented yet. The current binary is a placeholder that prints help.
- **IPC Model**: No IPC (UNIX sockets, etc.) should be reintroduced unless explicitly requested by the user. The goal is an autonomous daemon that primarily communicates via structured logs and a `status.json` file.
- **Features**: Features like "Preload" are currently in the **Planned** state. Do not document them as available features in help text or guides unless they are fully implemented.

## CLI and Runtime Model (Planned)
- **Autonomous Operation**: The daemon is designed to be autonomous. It observes system state and decides on actions locally. The Android application should not need to command it continuously.
- **Compact Flags**: Use compact, single-letter flags for features (e.g., `-p` for preload).
- **No IPC for Status**: Basic status reporting must not depend on IPC. The `corepolicy status` command reads from `/data/local/tmp/coreshift/status.json` directly.
- **Stable Substrate**: Keep the `low_level` module stable. 

## Implementation Priorities
1. **Correctness**: Ensure syscalls and resource ownership are handled safely in `low_level`.
2. **Minimalism**: Avoid complex IPC or heavy dependencies unless explicitly required.
3. **Transparency**: Use structured logging and the `status.json` file for observability.
