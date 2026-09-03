# android-core-runner

`android-core-runner` measures native ARM64 performance of the public `smt5-fusion-core::reverse_search::search` API on a physical Android device.

The runner uses the depth-4 corpus defined in [`../../docs/depth-4-performance-baseline.md`](../../docs/depth-4-performance-baseline.md): five target demons, each tested with a single external skill and with all supported Initial skills plus that external skill.

## Measured boundary

Data construction and `PlayerContext::prepare_direct_recipes` run before timing. Each sample measures one complete `search` call, including compressed route-space construction, exact route counting, default selection, and default-route replay. Destruction of the returned `RouteSelector` is outside the timed interval.

The CSV output contains median, P95, maximum latency, graph size, and exact route count for each case.

## Requirements

- Rust target `aarch64-linux-android`;
- Android NDK with an `aarch64-linux-android<API>-clang.cmd` linker;
- `adb` on `PATH`;
- a connected ARM64 Android device.

## Windows runner

From the repository root:

```powershell
./tools/android-core-runner/run.ps1 `
  -NdkHome E:\sgame\Android\android-ndk-r25c `
  -ApiLevel 29 `
  -Warmups 2 `
  -Samples 20
```

Run the same cases in reverse order to expose thermal or ordering bias:

```powershell
./tools/android-core-runner/run.ps1 `
  -NdkHome E:\sgame\Android\android-ndk-r25c `
  -ApiLevel 29 `
  -Warmups 2 `
  -Samples 20 `
  -Reverse
```

Use `-Serial <adb-serial>` when more than one device is connected.

The script installs the Rust target if necessary, builds the runner in Release mode, pushes it to `/data/local/tmp/android-core-runner`, executes it, and removes the remote binary afterward.

## Manual invocation

Set Cargo's Android linker, build the package, then push and execute it:

```text
cargo build --release --target aarch64-linux-android -p android-core-runner
adb push target/aarch64-linux-android/release/android-core-runner /data/local/tmp/android-core-runner
adb shell chmod 755 /data/local/tmp/android-core-runner
adb shell /data/local/tmp/android-core-runner --warmups 2 --samples 20
adb shell rm /data/local/tmp/android-core-runner
```

Run benchmarks on an otherwise idle and thermally stable device. Do not use these timings as browser/WASM results.
