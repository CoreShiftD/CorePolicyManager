# CorePolicyManager Rust Wrapper

This crate builds the `corepolicy` binary from the configured CoreShift-Policy
dependency tag for Android module packaging.

The wrapper delegates command behavior to public Policy APIs and exposes:

```text
corepolicy daemon
corepolicy preload-package <package>
corepolicy stats
corepolicy stats-reset
```
