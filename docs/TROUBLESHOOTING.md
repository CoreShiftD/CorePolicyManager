# Troubleshooting

## Daemon Not Starting

Check:

```bash
adb shell 'tail -n 100 /data/local/tmp/coreshift/service.log'
adb shell 'ls -l /data/adb/modules/coreshift_policy/system/bin/corepolicy'
```

If the binary is missing, rebuild the module and confirm ABI staging in the zip.

## Socket Not Found

Verify config and daemon state:

```bash
adb shell 'cat /data/local/tmp/coreshift/corepolicy.conf'
adb shell 'tail -n 100 /data/local/tmp/coreshift/service.log'
```

The default socket is abstract `@coreshift`; test with
`ABSTRACT-CONNECT:coreshift`.

## Stale Config

Updates preserve user config. If behavior does not match the packaged default,
inspect `/data/local/tmp/coreshift/corepolicy.conf` and merge new keys manually.

## Permission Errors

Android may deny optional install-directory scans. Policy reports those as skips
where possible. Check service logs and manual `preload-package` output.

## Module Update Behavior

The module replaces scripts and binaries but does not remove the runtime
directory. This preserves config and logs across updates and uninstall.
