# CorePolicyManager Rust Wrapper

This Rust crate is now a thin packaging wrapper around the released
`coreshift-policy` crate.

It keeps the `corepolicy` binary entrypoint for Android packaging and release
flows, but the reusable policy logic now lives in:

- `CoreShift-Core`
- `CoreShift-Engine`
- `CoreShift-Policy`

CorePolicyManager remains responsible for Android app integration, Magisk
packaging, assets, and release glue.
