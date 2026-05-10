# CorePolicyManager Rust Wrapper

This crate builds the `corepolicy` binary from the CoreShift-Policy `v0.3.0`
tag for Android module packaging.

The wrapper delegates command behavior to public Policy APIs and exposes:

```text
corepolicy daemon
corepolicy preload-package <package>
corepolicy stats
corepolicy stats-reset
```
