# Startup profiling

`FLEA_STARTUP_TRACE=1` enables passive startup timestamps on stderr. Normal launches leave tracing disabled. Records begin with `FLEA_STARTUP ` followed by JSON containing `pid`, `stage`, `start_ns`, `end_ns` and `duration_ms`. Timestamps use Linux `CLOCK_MONOTONIC`, matching compositor presentation feedback and `perf record --clockid mono`.

Application spans cover locale loading, the settings/device/theme preparation worker, GPUI construction, window creation and Shell construction. First-frame records distinguish submission, presentation and a presented complete List scene. The listing marker requires a current, visible, ready pane and the painted viewport rows; an empty listing is also valid. It does not claim coverage for Grid, Columns, covered panes, or input readiness. Compositors without monotonic presentation feedback cannot produce that marker. Presentation records use the compositor timestamp, not the time its callback reaches the application.

Spans nest and can overlap across threads. Do not sum all durations. Frame markers are emitted once per process, which matches the current one-process-per-main-window launch model. Unlike `FLEA_GPU_REPORT`, startup tracing does not drive scrolling or require a 10,000-entry fixture.

## Optimized build with symbols

Keep release optimization, LTO and the startup linker layout:

```sh
CARGO_PROFILE_RELEASE_DEBUG=1 CARGO_PROFILE_RELEASE_STRIP=none \
  RUSTFLAGS='-C symbol-mangling-version=v0' \
  cargo build --locked --release -p flea -p flea-gui

tools/flea-startup-order check target/release/flea-gui

FLEA_STARTUP_TRACE=1 FLEA_BIN="$PWD/target/release/flea" \
  target/release/flea --gui /absolute/folder 2>startup.log
```

Debug level 1 retains line tables and function symbols while preserving optimized code. Use `readelf -SW` to check `.debug_info`, `.debug_line`, `.symtab` and `.text.flea.startup`. Explicit v0 symbol naming is needed on toolchains whose default is legacy: the recorded startup section patterns name v0 symbols, and a nonempty but undersized startup section does not establish that the layout still matches.

## Detailed GPUI spans

[The dependency patch](../tools/startup-profile/gpui.patch) applies to the revision pinned in `Cargo.toml`, `04965e86d309e82eb0562e31a541032170379ba8`. It adds opt-in spans for Vulkan instance creation, adapter enumeration, device creation, test and window surface configuration, WGSL modules, each pipeline, transparency-triggered pipeline rebuilds, platform/text initialization and the initial Wayland roundtrip. It changes no renderer decisions.

Copy the pinned GPUI checkout to a scratch directory and apply the patch there with `git apply`. Do not edit Cargo's shared checkout. Override the repository's Git dependencies using an external Cargo configuration with `[patch."https://github.com/thisisgm/gpui-omarchy"]` entries pointing each used `gpui-pre*` package at its copied crate directory. Keep these overrides outside the user's normal Cargo configuration. Save `Cargo.lock` before the first build with overrides and restore it after profiling: local path patches change source entries in that file.

Pass `--config /absolute/profiling-patch.toml` to Cargo for the detailed build, with the same release/debug/v0 flags above. The first resolution of the local patches needs an unlocked build; subsequent builds may use `--locked`. Do not commit the path-based lockfile. Keeping the patch and its config outside normal dependency selection lets ordinary builds continue using the pinned upstream renderer.

## Sampling and comparison

Launch the built `flea --gui`, not just `flea-gui`, to retain launcher environment policy. Set `FLEA_BIN` to the built launcher so backend/helpers come from the same build. Record the monotonic launch timestamp before process creation and the compositor window-map event separately from the application markers.

```sh
perf record --clockid mono -F 997 -e cycles:u --call-graph dwarf,16384 \
  -o startup.data -- env FLEA_STARTUP_TRACE=1 \
  FLEA_BIN="$PWD/target/release/flea" target/release/flea --gui /absolute/folder
```

Filter the report to the GUI PID and the measured startup interval so helper CPU and later browsing do not get attributed to GUI startup. Use the full symbolized stack profile to inspect allocation and driver callers; sample percentages are CPU-cycle shares, not elapsed stage durations. Measure an interleaved trace-disabled control from the same executable to estimate instrumentation overhead. Separate warm-cache measurements from first-after-boot/update measurements.
