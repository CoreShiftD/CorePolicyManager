# Magisk Module

The built module zip contains:

- `module.prop`
- `customize.sh`
- `service.sh`
- `uninstall.sh`
- `corepolicy.conf`
- staged ABI binaries under `bin/arm64-v8a/` and `bin/armeabi-v7a/`

## Install Flow

`customize.sh` detects the device ABI and moves the matching staged binary to:

```text
system/bin/corepolicy
```

It then removes the staging `bin/` directory, applies module permissions, creates
`/data/local/tmp/coreshift`, and installs the default config only if the user does
not already have one.

## Service Flow

`service.sh` waits for boot completion, creates the runtime directory, exports:

```text
COREPOLICY_CONFIG=/data/local/tmp/coreshift/corepolicy.conf
```

Then it starts:

```text
corepolicy daemon
```

Logs are appended to `/data/local/tmp/coreshift/service.log` and rotated to
`service.log.1`. The service restarts the daemon with bounded backoff. It does
not currently write a pidfile.

If `/data/local/tmp/coreshift/debug` exists, the service also exports
`COREPOLICY_STDOUT_LOG=1` and `COREPOLICY_DEBUG_LOG=1`.
