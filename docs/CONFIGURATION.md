# Packaged Configuration

The Magisk package installs `corepolicy.conf` into the module work directory and
starts `corepolicy daemon` with `COREPOLICY_CONFIG` pointing at that file.

## Public Keys

The packaged example config exposes only the small user-facing surface:

```text
preload.enabled=true
preload.mode=auto
preload.foreground=true
preload.adaptive=false
preload.promote_artifacts=false
stats.enabled=false
log.enabled=false
game.enabled=false
game.preload_tier=hot
game.preload.promote_artifacts=true
game.downscale.enabled=false
game.downscale.factor=0.9
```

Unknown keys are rejected.

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
- `game.downscale` applies performance mode only.
- `log.enabled` controls all informational logging.
- Adaptive preload thresholds and stats flush tuning stay internal defaults.
