# Android Integration Model

The `high_level::android` module abstracts Android-specific platform services for use by policies and addons.

## System Properties

- **`getprop(key)` / `setprop(key, value)`**: Safe Rust wrappers for Android system properties. `getprop` utilizes the OS API bridged via `low_level::spawn::__system_property_get`, whereas `setprop` uses an inline `unsafe extern "C"` declaration for `__system_property_set`. This separates Android property APIs from standard command wrappers.

## Command Abstractions

Android shell utility wrappers:
- **`cmd(service, args, config)`**: Wraps Android's `/system/bin/cmd`.
- **`dumpsys(service, args, config)`**: Wraps `/system/bin/dumpsys`.

These builder functions yield `AndroidExecRequest` objects, which safely map down to pure execution specs handled by the Core engine without leaking implementation details into the pure layer.

## Inotify and Cgroups

When running with elevated privileges (e.g., in Preload mode), the daemon monitors system files like `/dev/cpuset/top-app/cgroup.procs` and `/data/system/packages.xml` using low-level `inotify` file descriptors. This avoids expensive CPU polling while tracking app focus changes and installations efficiently.
