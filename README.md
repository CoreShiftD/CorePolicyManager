# CorePolicyManager

CorePolicyManager v0.8.0 is the Magisk/product wrapper for the current
CoreShift-Policy release. It packages binaries, default config, and Magisk
scripts. It does not reimplement `corepolicy` logic.

```text
CorePolicyManager -> CoreShift-Policy -> CoreShift-Engine -> CoreShift-Core
```

## Documentation

- [Architecture](docs/ARCHITECTURE.md)
- [Magisk module](docs/MAGISK_MODULE.md)
- [Configuration](docs/CONFIGURATION.md)
- [Building](docs/BUILDING.md)
- [Installation](docs/INSTALLATION.md)
- [Device testing](docs/DEVICE_TESTING.md)
- [Troubleshooting](docs/TROUBLESHOOTING.md)

## Build

```bash
./scripts/build-magisk-module.sh
```

Output:

```text
dist/CoreShiftPolicy-v0.8.0.zip
```
