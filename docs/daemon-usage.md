# Daemon Operations & Usage

This guide covers running, controlling, and troubleshooting the CoreShift daemon directly.

## CLI Commands

The `coreshift` executable provides the following commands:
- `coreshift` (no args): Run daemon in default mode.
- `coreshift preload`: Run daemon with the Preload warmup addon enabled.
- `coreshift record <file>`: Run daemon and record trace to a file.
- `coreshift replay <file>`: Replay a recorded trace file locally.
- `coreshift help`: Show usage.

## Socket Configuration

Communication occurs over a UNIX domain socket.
- **Path**: `/data/local/tmp/coreshift/coreshift.sock`
- **Access**: Client peer credentials (UID) are verified upon connection via `SO_PEERCRED`.

## Runtime Logs

The daemon uses structured logging routed to the filesystem.
- **Core Engine Logs**: `/data/local/tmp/coreshift/core.log`
- **Addon Logs**: `/data/local/tmp/coreshift/addons/addon_<ID>.log`

## Control Triggers

You can control daemon verbosity and features dynamically by touching control files in the runtime directory (`/data/local/tmp/coreshift/control/`):
- **Enable Debug Logging**: `touch log_debug`
- **Enable Trace Logging**: `touch log_trace`
- **Enable Preload Addon**: `touch enable_preload`

## Common Startup Failures

1. **Address already in use**: 
   - *Cause*: A previous daemon instance crashed and left `coreshift.sock` bound. 
   - *Fix*: `rm /data/local/tmp/coreshift/coreshift.sock`
2. **Permission denied**:
   - *Cause*: Running without shell/root privileges to bind the socket or access the file system.
   - *Fix*: Run under `adb shell` or `su`.
3. **Reactor Wait Failed**:
   - *Cause*: The `low_level::reactor` epoll/kqueue descriptor threw an exception 10 times consecutively.
   - *Fix*: Check dmesg or OS limits on file descriptors.

## Operations Guide

When debugging or monitoring the daemon, use these techniques to assess its health:

### 1. Detecting Daemon Health
You can check if the daemon is currently running and responsive by using standard system tools:
```bash
# Check if the process is listed in the OS
ps -A | grep coreshift
```
If the daemon is frozen or deadlocked, it will fail to respond to IPC commands or write new lines to its log file.

### 2. Inspecting the Socket
Verify that the UNIX domain socket is successfully bound and listening:
```bash
# Ensure the socket exists
ls -l /data/local/tmp/coreshift/coreshift.sock

# Check if the process is holding it
lsof /data/local/tmp/coreshift/coreshift.sock
```
If the socket file exists but no process is holding it, the daemon has likely crashed, and you must remove the stale socket (`rm /data/local/tmp/coreshift/coreshift.sock`) before restarting.

### 3. Inspecting the PID
The daemon writes its Process ID to a file upon startup:
```bash
cat /data/local/tmp/coreshift/coreshift.pid
```
You can cross-reference this PID with `top` or `ps` to monitor CPU/memory consumption.

### 4. Inspecting the Logs
Tail the logs in real-time to watch state transitions, addon metrics, and potential errors:
```bash
# Tail the core daemon event loop and scheduler
tail -f /data/local/tmp/coreshift/core.log

# Tail a specific addon (e.g., Preload Addon #102)
tail -f /data/local/tmp/coreshift/addons/addon_102.log
```
Increase verbosity dynamically (by touching `log_trace` control trigger) if more context is needed.
