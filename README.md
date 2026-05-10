# CorePolicyManager

CorePolicyManager is the Magisk/product wrapper for `corepolicy` v0.2.0 from
CoreShift-Policy.

Layer order:

```text
CorePolicyManager -> CoreShift-Policy -> CoreShift-Engine -> CoreShift-Core
```

## Module behavior

- Installs the `corepolicy` binary into `system/bin`.
- Starts `corepolicy daemon` after boot.
- Exports `COREPOLICY_CONFIG=/data/local/tmp/coreshift/corepolicy.conf`.
- Installs a default key=value config only when the user does not already have one.
- Preserves `/data/local/tmp/coreshift` on update and uninstall.

The installer chooses the staged ABI binary, moves it to
`system/bin/corepolicy`, removes staging binaries from the module, and keeps the
runtime directory outside the module path so user configuration survives updates.

Default config is packaged at `packaging/magisk/corepolicy.conf`:

```text
socket=@coreshift
preload.enabled=true
preload.mode=auto
preload.foreground=true
preload.mmap_madvise_max_mb=128
preload.mmap_touch_max_mb=32
preload.asset_max_mb=512
preload.chunk_mb=16
```

## Service behavior

`packaging/magisk/service.sh` waits for Android boot completion, creates
`/data/local/tmp/coreshift`, exports `COREPOLICY_CONFIG`, and starts:

```bash
corepolicy daemon
```

Daemon stdout/stderr are appended to `/data/local/tmp/coreshift/service.log`.
The service rotates that file to `service.log.1` when it grows past the configured
limit. It restarts the daemon with bounded backoff after exits. The module does
not currently write a pidfile.

If `/data/local/tmp/coreshift/debug` exists, the service also exports
`COREPOLICY_STDOUT_LOG=1` and `COREPOLICY_DEBUG_LOG=1` before starting the
daemon.

## Install and update behavior

On first install, `customize.sh` copies the packaged `corepolicy.conf` to:

```text
/data/local/tmp/coreshift/corepolicy.conf
```

On update, an existing config at that path is preserved. Uninstall also leaves
`/data/local/tmp/coreshift` in place so logs and user config remain available.

## Build

Requires Rust, Android NDK, and `zip`.

```bash
./scripts/build-magisk-module.sh
```

Output:

```text
dist/CoreShiftPolicy-v0.2.0.zip
```

## Device checks

After flashing, verify the config:

```bash
adb shell 'cat /data/local/tmp/coreshift/corepolicy.conf'
```

Query the daemon socket:

```bash
adb shell "printf 'GET\n' | socat - ABSTRACT-CONNECT:coreshift"
```

Run a manual preload:

```bash
adb shell 'corepolicy preload-package com.example.app'
```

Read service logs:

```bash
adb shell 'tail -n 100 /data/local/tmp/coreshift/service.log'
```
