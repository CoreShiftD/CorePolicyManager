# Packaged Configuration

The Magisk package installs `corepolicy.conf` into the module work directory and
starts `corepolicy daemon` with `COREPOLICY_CONFIG` pointing at that file.

## Public Keys

The packaged example config exposes only the small user-facing surface:

```text
preload.mode=auto
preload.adaptive=true
preload.promote_artifacts=true
stats.enabled=true
log.enabled=true
game.enabled=true
game.preload_tier=hot
game.preload.promote_artifacts=true
game.downscale.enabled=true
game.downscale.factor=0.9
```

Unknown keys are rejected.

`preload.mode=off` disables automatic foreground preload. Other modes keep
foreground preload active as the daemon's main behavior.

## Derived Files

Policy derives its daemon home from the parent directory of `corepolicy.conf`.
These files are internal and are not configurable:

```text
<daemon-home>/corepolicy.conf
<daemon-home>/gamelist.txt
<daemon-home>/stats.tsv
<daemon-home>/game_interventions.tsv
```

## Socket

The daemon uses the internal abstract socket `@coreshift`.

Local clients are authorized with Unix peer credentials and are accepted only
when the peer UID matches the daemon process effective UID. There is no
configurable UID allowlist.

## Notes

- `game.enabled` uses exact package matches from `gamelist.txt`.
- Confirmed game foreground sessions also trigger an internal one-shot
  `/system/bin/cmd package kill-all` trim when `game.enabled=true`.
- `game.downscale` applies performance mode only.
- `log.enabled` controls all informational logging.
- Adaptive preload thresholds stay internal defaults.
- Stats are flushed on every confirmed foreground change when enabled and stored
  as `package`, `start_time_ms`, `last_time_ms`, `foreground_ms`, `sessions`.
- `game_interventions.tsv` keeps its legacy filename for compatibility, but it
  stores managed game downscale state.
