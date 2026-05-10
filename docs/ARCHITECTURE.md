# CorePolicyManager Architecture

CorePolicyManager packages CoreShift-Policy for Magisk/product deployment. It is
not a second policy engine.

## Role

Manager owns:

- Rust wrapper crate for the `corepolicy` binary.
- Android cross-build scripts.
- Magisk module layout.
- ABI binary staging.
- Default config packaging.
- Install/update scripts.
- Boot-time service startup.

Manager does not own:

- Foreground detection logic.
- Package preload behavior.
- Socket protocol semantics.
- CoreShift policy rules.

Those live in CoreShift-Policy and its lower layers.

## Layering

```text
CorePolicyManager
  packages and starts corepolicy
CoreShift-Policy
  Android daemon and preload policy
CoreShift-Engine
  reusable mechanisms
CoreShift-Core
  syscall/filesystem primitives
```
