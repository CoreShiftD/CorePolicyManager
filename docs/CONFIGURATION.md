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
log.game=false
game.enabled=false
game.list_path=/data/local/tmp/coreshift/gamelist.txt
game.preload_tier=hot
game.intervention.enabled=false
game.intervention.apply_on_start=true
game.intervention.watch_list=false
game.intervention.revert_on_remove=false
game.intervention.state_path=/data/local/tmp/coreshift/game_interventions.tsv
game.intervention.mode=performance
game.intervention.performance_downscale=0.9
game.intervention.battery_downscale=0.7
game.intervention.standard_downscale=1.0
game.intervention.dry_run=false
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
foreground preload, stats, and game intervention info logs quiet by default. Set
`log.enabled=true` and the needed category key (`log.foreground`, `log.preload`,
`log.stats`, or `log.game`) to enable that category. Service lifecycle messages
still go to `service.log`.

Game-list mode is disabled by default. When `game.enabled=true`, Policy reads
`game.list_path` and treats exact package matches as games for preload tier
selection only. `game.preload_tier=warm` applies warm adaptive preload behavior;
`game.preload_tier=hot` applies hot behavior. Non-game apps still use the normal
adaptive stats/default policy.

Game interventions are also opt-in. They run only when both `game.enabled=true`
and `game.intervention.enabled=true`, and only for packages loaded from
`gamelist.txt`. Policy configures Android Game Mode overlays with
`device_config put game_overlay <package> ...` and selects the configured mode
with `cmd game mode standard|performance|battery <package>`. With
`game.intervention.apply_on_start=true`, the daemon applies interventions once at
startup after loading the gamelist; it does not reapply on foreground changes and
does not force-stop or restart apps. `game.intervention.dry_run=true` reports the
commands without executing them. Downscale values must be in the `0.5` through
`1.0` range.

Downscaling takes effect only after the target app restarts. Support depends on
ROM, OEM, and Android version. To disable/revert one package:

```sh
device_config delete game_overlay <package>
cmd game mode standard <package>
```

`corepolicy game-revert <package>` reverts only when the package exists in the
managed state file.

`game.intervention.watch_list=true` enables an inotify watch on the parent
directory of `game.list_path`. Atomic updates are supported: write a temp file
such as `gamelist.txt.tmp`, then rename it over `gamelist.txt`. The daemon
reacts to `MOVED_TO`, `CLOSE_WRITE`, and `DELETE` for `gamelist.txt`; it does
not poll and does not do gamelist work on foreground changes.

`game.intervention.state_path` stores CoreShift-managed interventions as TSV:

```text
package	mode	overlay_hash	applied_ms
```

When `game.intervention.revert_on_remove=true`, removed packages are reverted
only if they appear in this managed state file. CoreShift does not delete
overlays for packages it did not manage. If `gamelist.txt` is missing, the list
is treated as empty; managed packages are reverted only when
`revert_on_remove=true`.

The packaged default `gamelist.txt` is installed to:

```text
/data/local/tmp/coreshift/gamelist.txt
```

The list format is one full package name per line. Blank lines and lines
starting with `#` are ignored. User edits are preserved on update. There is no
APK manifest parsing, string scanning, auto-detection, wildcard matching, or
network access at runtime. The external Encore list is only used as the packaged
seed/reference.

Use `corepolicy stats` to print the current TSV and `corepolicy stats-reset` to
remove the configured stats file/temp file and request a running daemon to clear
in-memory stats. Dirty stats flush after `stats.flush_every_changes` foreground
changes or after `stats.flush_interval_s` seconds.

CorePolicyManager v0.5.0 builds against CoreShift-Policy v0.5.0 so packaged
config and binary support match.

The service exports `COREPOLICY_CONFIG` to point at the runtime config path
before starting `corepolicy daemon`.

## Update Behavior

Module updates do not overwrite an existing runtime config or gamelist. If a new
default is needed, compare the packaged `corepolicy.conf` or `gamelist.txt` with
the preserved runtime file and merge intentionally.
