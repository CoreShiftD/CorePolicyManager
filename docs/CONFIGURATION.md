# Configuration

The module packages a default key=value config at:

```text
packaging/magisk/corepolicy.conf
```

On first install, it is copied to:

```text
/data/local/tmp/coreshift/corepolicy.conf
```

Existing user config is preserved on update.

## Default Config

```text
socket=@coreshift
preload.enabled=true
preload.mode=auto
preload.foreground=true
preload.mmap_madvise_max_mb=128
preload.mmap_touch_max_mb=32
preload.asset_max_mb=512
preload.chunk_mb=16
preload.adaptive.enabled=false
preload.adaptive.min_sessions=3
preload.adaptive.min_foreground_ms=300000
preload.adaptive.hot_sessions=10
preload.adaptive.hot_foreground_ms=3600000
stats.enabled=false
stats.flush_every_changes=10
stats.flush_interval_s=0
stats.path=/data/local/tmp/coreshift/stats.tsv
log.enabled=false
log.foreground=false
log.preload=false
log.stats=false
```

Adaptive preload is disabled by default. When enabled, CoreShift-Policy uses
local stats to keep unknown/cold apps on readahead and promote `.so` files to
`MmapMadvise` for warm/hot apps. APK, split APK, DM, and OAT files stay on
readahead in this release.

Stats are local foreground usage counters written by CoreShift-Policy when
enabled. They collect package, uid, sessions, foreground_ms, and last_seen_ms in
TSV at `stats.path`. No network upload and no JSON are used.
These keys require a `corepolicy` release with local stats and adaptive preload
support.

Informational daemon logs are config-controlled. `log.enabled=false` keeps
foreground preload and stats info logs quiet by default. Set `log.enabled=true`
and the needed category key (`log.foreground`, `log.preload`, or `log.stats`) to
enable that category. Service lifecycle messages still go to `service.log`.

Use `corepolicy stats` to print the current TSV and `corepolicy stats-reset` to
remove the configured stats file/temp file and request a running daemon to clear
in-memory stats. Dirty stats flush after `stats.flush_every_changes` foreground
changes or after `stats.flush_interval_s` seconds.

CorePolicyManager v0.2.0 builds against the configured CoreShift-Policy
dependency tag so packaged config and binary support match.

The service exports `COREPOLICY_CONFIG` to point at the runtime config path
before starting `corepolicy daemon`.

## Update Behavior

Module updates do not overwrite an existing runtime config. If a new default is
needed, compare the packaged `corepolicy.conf` with the preserved runtime file
and merge intentionally.
