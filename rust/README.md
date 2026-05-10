# CorePolicyManager Rust Wrapper

This crate builds the `corepolicy` binary from `coreshift-policy` v0.2.0 for
Android module packaging.

The wrapper keeps the package layout independent from the Policy repository while
delegating command behavior to the same public Policy APIs used by the standalone
`corepolicy` binary:

```text
corepolicy daemon
corepolicy preload-package <package>
```
