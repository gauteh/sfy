# Copilot instructions for sfy

Monorepo for the SFY wave buoy. Firmware lives in `sfy4-buoy/` (current
generation; `sfy3-buoy/` is the previous one).

## Building `sfy4-buoy` for the embedded (thumbv7em-none-eabihf) target

Requires these env vars for bindgen (used by `ambiq-hal-sys`) to find the ARM
cross headers, otherwise the build fails with a missing `string.h` error:

```
export LIBCLANG_PATH=/usr/lib/llvm-14/lib
export BINDGEN_EXTRA_CLANG_ARGS="--target=thumbv7em-none-eabihf -I/usr/lib/gcc/arm-none-eabi/13.2.1/include"
```

## Host tests

Run with `make host-test` (in `sfy4-buoy/`), which loops `cargo test` across
several feature combinations (`testing`, `raw`, `fir`, `spectrum`,
`simulate-egps`, etc.) — always run this rather than a single `cargo test`
invocation when validating changes.

## Terminology: "batch" (egps) vs "spectrum" (axl)

These are two unrelated concepts that both used to be called "spectrum" —
don't conflate them:

- **egps batch**: a high-rate raw-sample burst from the external GPS
  (`sfy::gps::duty`), sent as `egpsb.qo`. Duration/period configured via the
  `EGPS_BATCH_DURATION`/`EGPS_BATCH_PERIOD` build-time env vars.
- **axl/IMU spectrum** (`spectrum` Cargo feature, `sfy::waves::welch`): FFT/Welch
  spectrum of wave motion. Its length is **hardcoded to 20 minutes**
  (`src/waves/welch.rs`, `Welch::is_full`) — not configurable via env var,
  unlike the egps batch duration.

## Local dev patches

`sfy4-buoy/Cargo.toml` depends on `blues-notecard` from the `env` branch of
`github.com/gauteh/notecard-rs` (has the `Notecard::env()` API used by
`src/note.rs`). There's a commented-out `[patch]` entry pointing it at a
local checkout (`../../../../dev/embedded/notecard-rs`) for local dev use —
leave it commented out in commits so CI/remote builds fetch from git.
