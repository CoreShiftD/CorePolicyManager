# Device Testing

## Check Config

```bash
adb shell 'cat /data/local/tmp/coreshift/corepolicy.conf'
```

Expected config includes `socket=@coreshift` and preload keys.

## Check Logs

```bash
adb shell 'tail -n 100 /data/local/tmp/coreshift/service.log'
```

Expected logs include daemon start lines and foreground/preload messages when
activity occurs.

## Runtime Control

```bash
adb shell 'corepolicy status'
adb shell 'corepolicy restart'
adb shell 'corepolicy watch'
adb shell 'corepolicy stats'
adb shell 'corepolicy stats reset'
adb shell 'corepolicy gamelist'
```

`status` prints daemon key=value lines. `restart` prints `restarting`; Magisk
`service.sh` starts the daemon again. `watch` keeps running and prints foreground
package changes until interrupted. `stats` prints foreground counters;
`stats reset` clears them. `gamelist` prints installed packages that also appear
in the configured gamelist.

## Check Binary

```bash
adb shell 'ls -l /system/bin/corepolicy /data/adb/modules/coreshift_policy/system/bin/corepolicy'
```

At least the module path should exist. Device overlay behavior determines what
is visible through `/system/bin`.
