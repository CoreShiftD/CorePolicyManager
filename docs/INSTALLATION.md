# Installation and Updates

Flash the generated `dist/CoreShiftPolicy-<version>.zip` through Magisk. For
CorePolicyManager v0.9.0 this is `dist/CoreShiftPolicy-v0.9.0.zip`.

## First Install

On first install:

- The matching ABI binary is installed as `system/bin/corepolicy`.
- `/data/local/tmp/coreshift` is created.
- Default `corepolicy.conf` is installed if no runtime config exists.
- Default `gamelist.txt` is installed if no runtime gamelist exists.
- `service.sh` starts the internal `corepolicy daemon` after boot.

## Update

On update:

- Existing `/data/local/tmp/coreshift/corepolicy.conf` is preserved.
- Existing `/data/local/tmp/coreshift/gamelist.txt` is preserved.
- Existing logs under `/data/local/tmp/coreshift` are preserved.
- The module binary and scripts are replaced by the new zip contents.

## Uninstall

`uninstall.sh` intentionally leaves `/data/local/tmp/coreshift` in place so user
config and logs remain available.
