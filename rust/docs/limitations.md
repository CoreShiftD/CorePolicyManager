# Known Limitations and TODO

CoreShift is experimental. Several areas remain under active development or carry known caveats.

## Incomplete Features

- **JNI Bridge**: The daemon currently communicates solely over the UNIX domain socket. A direct in-process JNI bridge is not yet implemented.
- **Configuration Persistence**: The daemon currently uses volatile file-based triggers (`touch /data/local/tmp/coreshift/control/enable_preload`) rather than reading from a persistent `settings.json` or SQLite store.

## Experimental Surfaces

- **Preload Addon Cache Invalidation**: Cgroups inotify polling limits remain untested on older Linux kernel variants.
- **IPC Protocol**: Payload structures use manually framed binary schemas (e.g., `1` for JSON, `2` for u64 IDs). This is slated for replacement with pure bincode or capnproto later.

## Portability Caveats

- **System Property APIs**: The `high_level::android` wrappers for `setprop` and `getprop` use `libc` and target-specific `__system_property_set`. These will cause build failures or runtime panics if ported unmodified to non-Android Linux devices.
