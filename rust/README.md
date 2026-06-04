# CorePolicyManager Rust Wrapper

This crate builds the `corepolicy` binary from the configured CoreShift-Policy
dependency tag for Android module packaging.

The wrapper delegates command behavior to public Policy APIs and exposes:

```text
corepolicy status
corepolicy watch
corepolicy restart
corepolicy stats
corepolicy stats reset
corepolicy gamelist
corepolicy daemon
```

`corepolicy daemon` is service-internal for Magisk startup.
