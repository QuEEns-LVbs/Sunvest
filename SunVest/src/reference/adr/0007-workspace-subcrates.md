# ADR 0007: Workspace Subcrates Compiled Into Sunvest-core

## Status

Accepted

## Context

Sunvest is adding three competitive layers (ZK circuit builders, session UX, game
testing sandbox). They need compile-time isolation without publishing separate
crates.io packages, so download metrics stay unified under `Sunvest-core`.

## Decision

1. Add a Cargo workspace with three internal members under `internal/`:
   - `Sunvest-core-circuits`
   - `Sunvest-core-session`
   - `Sunvest-core-test`

2. Each internal member sets `publish = false`.

3. Each layer's implementation lives in `src/{circuits,session,test}/inner.rs`.
   The public modules `include!` that file; internal workspace members point their
   `[lib] path` at the same `inner.rs` for isolated `cargo check -p` runs. This avoids:
   - circular dependencies (session needs `auth` from the same crate)
   - `cargo publish` failures on unpublished path deps
   - missing files in the published tarball

4. Public API:
   - `Sunvest_core::circuits` - always available
   - `Sunvest_core::session` - always available
   - `Sunvest_core::test` - `testutils` feature only

5. The test sandbox uses `no_std` + `alloc`, not `std`. It runs in Soroban
   `testutils` environments the same way contract tests do today.

## Consequences

- One `cargo add Sunvest-core` for all capabilities
- Internal folders can still be checked with `cargo check -p Sunvest-core-session`
- `cargo publish` ships `internal/**` sources inside the `Sunvest-core` tarball
- Feature `testutils` keeps sandbox code out of contract WASM builds