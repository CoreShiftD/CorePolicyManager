# Magisk Module

The built module zip contains:

- `module.prop`
- `customize.sh`
- `service.sh`
- `uninstall.sh`
- `corepolicy.conf`
- `gamelist.txt`
- staged ABI binaries under `bin/arm64-v8a/` and `bin/armeabi-v7a/`

## Install Flow

`customize.sh` detects the device ABI and moves the matching staged binary to:

```text
system/bin/corepolicy
```

It then removes the staging `bin/` directory, applies module permissions, creates
`/data/local/tmp/coreshift`, and installs the default config and gamelist only if
the user does not already have them. User edits are preserved on update.

## Service Flow

`service.sh` waits for boot completion, creates the runtime directory, waits
for SystemUI readiness, and exports:

```text
COREPOLICY_CONFIG=/data/local/tmp/coreshift/corepolicy.conf
```

Then it starts:

```text
corepolicy daemon
```

`corepolicy daemon` is service-internal; use `corepolicy status`,
`corepolicy watch`, `corepolicy restart`, `corepolicy stats`,
`corepolicy stats reset`, and `corepolicy gamelist` for normal device checks.
Logs are appended to `/data/local/tmp/coreshift/service.log` and rotated to
`service.log.1`. The service restarts the daemon with bounded backoff. It does
not currently write a pidfile. Optional daemon informational logs are controlled
by `log.enabled` in `/data/local/tmp/coreshift/corepolicy.conf`; there is no
debug-file control path.

After `service.sh` starts the daemon, Policy may run a one-shot boot tuning
worker when `boot.tuning=true`. That worker is best-effort, uses direct Android
property APIs where available plus absolute `/system/bin/*` command paths,
reuses or generates `/data/local/tmp/coreshift/wm.txt` for window logging tag
cleanup, and exits without blocking the foreground watcher. Same-boot reruns are
gated by the volatile property `debug.coreshift.boot_tuning=completed`.

Game-list classification is controlled by `game.*` keys in `corepolicy.conf` and
the user-editable `/data/local/tmp/coreshift/gamelist.txt`. It affects preload
tier selection, an internal one-shot game trim on confirmed game foreground
sessions, and, when explicitly enabled, Android Game Mode downscale.
Policy does not parse APK manifests or fetch lists at runtime. Game downscale
applies once at daemon start, only to gamelist packages, uses performance mode
only, and downscale changes require app restart. Optional inotify watching
reconciles atomic gamelist replacements. Managed reverts only touch packages
recorded in `/data/local/tmp/coreshift/game_interventions.tsv`, which keeps its
legacy filename for compatibility while storing managed game downscale state.
ROM/OEM/Android support varies. CoreShift intentionally does not apply package
idle sweeps, backup disabling, statusbar disable flags, or memory-factor
changes by default.
