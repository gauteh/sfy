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
several feature combinations (`testing`, `raw`, `fir`, `spectrum`, etc.) —
always run this rather than a single `cargo test` invocation when validating
changes.

## Building a test binary for egps duty-cycle field tests

Default params used for live duty-cycle field tests (40 min between egps
batches, 10 min position fix interval, 20 min sync period), release mode,
from `sfy4-buoy/` (with the cross-compile env vars above set):

```
make T=r bin SYNC_PERIOD=20 EGPS_BATCH_PERIOD=2400 EGPS_POSITION_INTERVAL=600
```

Produces `target/sfy4-main.bin`.

For indoor/bench testing where the MAX-M10S can't get a real satellite fix
(e.g. no antenna, weak signal), add `accept-no-egps-fix` (via `CARGO_FLAGS`,
since `make bin` doesn't take a `FEATURES=` var) to relax egps fix
acquisition so batches start anyway -- exercises the batch/duty-cycle
pipeline without a real fix. Not for field/deploy builds:

```
make T=r bin SYNC_PERIOD=20 EGPS_BATCH_PERIOD=2400 EGPS_POSITION_INTERVAL=600 CARGO_FLAGS="--features accept-no-egps-fix"
```

### Short-cycle debug build (reproducing duty-cycle bugs faster)

For debugging duty-cycle state-machine issues (e.g. a wedge that only
clears on reboot) it helps to shrink the whole cycle so failures show up in
minutes instead of hours. `EGPS_BATCH_DURATION` (batch length, seconds) is
build-time configurable just like the other `EGPS_*` knobs -- no source
edits needed. Example: 5 min batches, 5 min breaks, 5 min position wakes,
2 min dwell, with a 15 min status/sync period so you still get status logs
somewhat promptly without spamming syncs:

```
make T=r bin SYNC_PERIOD=15 EGPS_BATCH_DURATION=300 EGPS_BATCH_PERIOD=600 EGPS_POSITION_INTERVAL=300 EGPS_POSITION_DWELL=120
```

## Analyzing device data with `sfy-processing`

`sfy-processing`'s Python tooling (`Hub`/`SfyBuoy`, `SFY_READ_TOKEN`/
`SFY_SERVER` env vars) requires the `sfy` mamba/conda environment (numpy
etc. aren't in the base env) — run it via:

```
mamba run -n sfy python3 ...
```

## Terminology: "batch" (egps) vs "spectrum" (axl)

These are two unrelated concepts that both used to be called "spectrum" —
don't conflate them:

- **egps batch**: a high-rate raw-sample burst from the external GPS
  (`sfy::gps::duty`), sent as `egpsb.qo`. 20 min duration by default
  (`EGPS_BATCH_DURATION_S`, build-time configurable via the
  `EGPS_BATCH_DURATION` env var); period configured via the
  `EGPS_BATCH_PERIOD` build-time env var. On a position-only wake
  (no batch due), a brief post-fix sample (`EGPS_POSITION_SAMPLE_MS`) still
  streams so something is queued for the next sync.
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
