---
phase: 01-foundation-mvp-slice
reviewed: 2026-05-18T00:00:00Z
depth: standard
files_reviewed: 16
files_reviewed_list:
  - crates/alloc-bench-aggregator/src/main.rs
  - crates/alloc-bench-cli/build.rs
  - crates/alloc-bench-cli/src/allocator.rs
  - crates/alloc-bench-cli/src/build_info.rs
  - crates/alloc-bench-cli/src/main.rs
  - crates/alloc-bench-cli/src/run.rs
  - crates/alloc-bench-cli/tests/multithread_smoke.rs
  - crates/alloc-bench-core/src/harness.rs
  - crates/alloc-bench-core/src/lib.rs
  - crates/alloc-bench-core/src/metrics/env.rs
  - crates/alloc-bench-core/src/metrics/mod.rs
  - crates/alloc-bench-core/src/metrics/rusage.rs
  - crates/alloc-bench-core/src/metrics/statm.rs
  - crates/alloc-bench-core/src/output.rs
  - crates/alloc-bench-core/src/scenarios/mod.rs
  - crates/alloc-bench-core/src/scenarios/multithread.rs
findings:
  critical: 3
  warning: 9
  info: 6
  total: 18
status: issues_found
---

# Phase 01: Code Review Report

**Reviewed:** 2026-05-18
**Depth:** standard
**Files Reviewed:** 16
**Status:** issues_found

## Summary

Phase 1 establishes a walking-skeleton workspace with allocator selection, build-metadata
injection, a harness with HDR-histogram latency capture, a multithread allocation scenario,
and a smoke test. The unsafe code in `metrics/rusage.rs` is sound; the `compile_error!`
mutual-exclusion guard in `allocator.rs` is correct; `std::thread::scope` use in
`scenarios/multithread.rs` does not introduce data races.

However, the implementation has **three Critical defects** that affect benchmark validity
or cause silent failures: (1) the `Bimodal` size distribution can return values *outside*
`[size_min, size_max]`, contaminating measurements; (2) panics in worker threads are
silently swallowed, masking failures and reporting bogus throughput; (3) the build script
does not register `.git/HEAD` / `.git/index` for change tracking, so embedded git SHA and
"dirty" status go stale across rebuilds (compromising the project's stated reproducibility
constraint).

Additional Warnings cover input validation gaps (`size_min > size_max`, zero size buffers
panicking via `b[size/2]`), HDR-histogram bound overflow in long ticks, semantic
mismatch between latency-per-tick and "ops/s" labelling, an `eprintln!` banner that
fires before clap parses `--help`, an unhelpful error message for `getrusage` failure
(no errno), and a missing `sysconf` failure check in `statm.rs`. Info items cover style
and minor robustness improvements.

## Critical Issues

### CR-01: `SizeDist::Bimodal` returns sizes outside the user-requested range

**File:** `crates/alloc-bench-core/src/scenarios/multithread.rs:52-57`

**Issue:** `Bimodal` returns `min.max(16)` for the small-bucket case. If the user passes
`--size-min 8`, the worker allocates 16-byte buffers, **violating the stated bounds**.
This contaminates the measurement (the user is benchmarking a different size class than
they asked for) and is inconsistent with `Uniform`/`Pareto` which respect `min`. The
`16` magic number appears to be an unrelated workaround that leaked into the size selector.

**Fix:**
```rust
SizeDist::Bimodal => {
    if rng.gen::<f32>() < 0.9 { min } else { max }
}
```
If a sane minimum (e.g., header-size guard) is genuinely required, validate it once
at config parse time in `MultithreadConfig::new` and reject `size_min < 16` with a
helpful error — don't silently rewrite user input inside the hot path.

### CR-02: Worker-thread panics are silently swallowed; bogus throughput reported

**File:** `crates/alloc-bench-core/src/scenarios/multithread.rs:103-107`

**Issue:**
```rust
for h in handles {
    if let Ok(bag) = h.join() {
        all.push(bag);
    }
}
```
If any worker panics (e.g., due to OOM, a bug, or the zero-size buffer panic in WR-02),
`h.join()` returns `Err(_)` and is silently dropped. The harness continues, records the
tick latency for the *partial* outcome, divides by `measurement_s`, and reports a
"successful" benchmark with throughput numbers derived from a broken run. This is a
correctness defect for an experiment harness whose entire value depends on result
trustworthiness.

**Fix:**
```rust
for h in handles {
    match h.join() {
        Ok(bag) => all.push(bag),
        Err(payload) => std::panic::resume_unwind(payload),
    }
}
```
Or, if propagating panics through `tick()` is structurally awkward, change the trait so
`tick()` returns `Result<Box<dyn SinkValue>>` and have the harness fail loudly.

### CR-03: `build.rs` does not declare `.git/HEAD` / `.git/index` as rerun triggers; embedded git SHA goes stale

**File:** `crates/alloc-bench-cli/build.rs:59-60`

**Issue:** The script reads `git rev-parse HEAD` and `git status --porcelain` but only
registers:
```rust
println!("cargo:rerun-if-changed=build.rs");
println!("cargo:rerun-if-env-changed=CARGO_ENCODED_RUSTFLAGS");
```
After the first build, Cargo caches the env vars and skips the script. If the user
commits, switches branches, or modifies a tracked file, `BUILD_GIT_SHA` and
`BUILD_GIT_DIRTY` are *not* refreshed unless `build.rs` itself is touched. The same
applies to `BUILD_TIMESTAMP`. This violates the project constraint
*"Every result is reproducible, environment-labelled"* — the JSON output may carry the
SHA of an old commit while running freshly compiled code.

**Fix:** Add these directives so Cargo reruns the script when the working tree changes:
```rust
println!("cargo:rerun-if-changed=../../.git/HEAD");      // branch switch
println!("cargo:rerun-if-changed=../../.git/index");     // staged changes
// For tracking dirty status across the whole tree, the simplest pragmatic option
// is "always rerun":
println!("cargo:rerun-if-changed=NULL");
```
Be aware: `.git` paths are relative to the package directory; verify the path resolves
in this workspace layout. If you accept "always rerun" for forensic accuracy, document
the trade-off (small build-time cost) in `build.rs`.

## Warnings

### WR-01: HDR-histogram tick-duration recording mislabels measurement units

**File:** `crates/alloc-bench-core/src/harness.rs:51,61-62,73-74,82-90`

**Issue:** Each `tick()` in `Multithread` performs `threads * objects` allocations
(default 16 * 100,000 = 1.6M allocs), yet the harness records *one* sample per tick:
```rust
let elapsed_ns = t0.elapsed().as_nanos() as u64;
hist.record(elapsed_ns.max(1))?;
```
Then computes `throughput_ops_per_s = samples_count / measurement_s` where `samples_count`
is *the number of ticks, not the number of allocations*. The `latency_ns.{p50,p95,...}`
values are per-tick fork/join durations, not per-allocation latencies. The downstream
report will read "p50 = 200ms" and "throughput = 5 ops/s" — neither of which matches
how the project advertises itself ("memory allocator benchmark"). This is a measurement
modeling defect that propagates into every Run record.

**Fix:** Either (a) shrink the unit of work — make `tick()` perform a single allocation,
push it into a thread-local bag retained across ticks, and free at teardown; or (b) keep
the current batched design and rename the schema fields to `tick_latency_ns` and
`ticks_per_s`, and add `allocations_per_tick` so consumers can derive an allocation rate.
Current naming will mislead users.

### WR-02: Zero-size buffer in `multithread.rs` panics on indexed write

**File:** `crates/alloc-bench-core/src/scenarios/multithread.rs:94-97`

**Issue:**
```rust
let mut b: Box<[u8]> = vec![0u8; size].into_boxed_slice();
b[size / 2] = 0xAB;
```
With `size == 0`, `b` is empty and `b[0]` panics. CLI input `--size-min 0 --size-max 0`
(or any combo that makes `gen_range`/`Pareto` produce 0) crashes the worker. Combined
with CR-02, this becomes a silent panic with bogus results. The project lacks input
validation in `MultithreadConfig` to reject `size_min == 0`.

**Fix:** Validate at config construction time, e.g.,
```rust
impl MultithreadConfig {
    pub fn validated(self) -> anyhow::Result<Self> {
        anyhow::ensure!(self.size_min >= 1, "size_min must be >= 1");
        anyhow::ensure!(self.size_min <= self.size_max, "size_min must be <= size_max");
        anyhow::ensure!(self.threads >= 1, "threads must be >= 1");
        anyhow::ensure!(self.objects >= 1, "objects must be >= 1");
        Ok(self)
    }
}
```
Plumb through `Multithread::new` or call it in `run::run_multithread` before constructing
the scenario.

### WR-03: `size_min > size_max` causes `gen_range` panic with no upstream check

**File:** `crates/alloc-bench-core/src/scenarios/multithread.rs:51,53`

**Issue:** `rng.gen_range(min..=max)` panics if `min > max`. CLI accepts both flags
with no relational check. Hits CR-02 → silent failure.

**Fix:** Same as WR-02 — validate at config construction; reject with an `anyhow::Error`
instead of crashing a worker thread.

### WR-04: HDR-histogram upper bound (60s) can be exceeded by a heavy `tick()`, terminating the run

**File:** `crates/alloc-bench-core/src/harness.rs:51,62`

**Issue:** `Histogram::<u64>::new_with_bounds(1, 60_000_000_000, 3)` rejects any sample
above 60s. Combined with the per-tick fork/join model (WR-01), a saturated CI runner or
an aggressive `--threads --objects` combination can plausibly produce a tick longer than
60s. `hist.record(...)?` then **terminates the entire run** mid-measurement and
discards all collected data. There is no graceful saturation.

**Fix:** Use `record_correct` with a saturating fallback, or pre-clip:
```rust
let v = elapsed_ns.max(1).min(60_000_000_000);
hist.record(v)?;
```
And/or widen the upper bound (e.g., `300_000_000_000` for 5min). Document the chosen
ceiling in a comment so reviewers understand the trade-off (memory vs head-room).

### WR-05: `metrics/statm.rs` does not check `sysconf(_SC_PAGESIZE)` for `-1` failure

**File:** `crates/alloc-bench-core/src/metrics/statm.rs:12-13`

**Issue:**
```rust
let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) } as u64;
Ok(resident_pages * page_size / 1024)
```
`sysconf` returns `-1` on failure (`errno` set). Casting `-1` (i64) to `u64` yields
`0xFFFF_FFFF_FFFF_FFFF`; the multiplication overflows or returns a bogus huge value, and
the resulting `rss_kb` is meaningless. While `_SC_PAGESIZE` failing is highly unlikely on
Linux, the code still needs a guard for soundness on weird kernels / sandboxes / seccomp
filters.

**Fix:**
```rust
let raw = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
anyhow::ensure!(raw > 0, "sysconf(_SC_PAGESIZE) failed: {}", std::io::Error::last_os_error());
let page_size = raw as u64;
```

### WR-06: `getrusage` failure path does not capture `errno`

**File:** `crates/alloc-bench-core/src/metrics/rusage.rs:6`

**Issue:** `anyhow::ensure!(ret == 0, "getrusage failed");` discards the OS error. On
sandboxes where `getrusage(RUSAGE_SELF)` is restricted, a debugging session would have
to guess the cause.

**Fix:**
```rust
anyhow::ensure!(
    ret == 0,
    "getrusage failed: {}",
    std::io::Error::last_os_error()
);
```

### WR-07: Banner is printed before `Cli::parse`, leaking onto `--help`/`--version`

**File:** `crates/alloc-bench-cli/src/main.rs:74-78`

**Issue:**
```rust
fn main() -> Result<()> {
    print_version_banner();      // unconditional eprintln!
    allocator::assert_mutual_exclusion();
    let cli = Cli::parse();      // exits after this on --help / --version
    ...
}
```
Running `alloc-bench-cli --help` prints the banner to stderr, then clap prints help
to stdout, then exits. Tests using `assert_cmd` to assert on `--help` will see the
banner in stderr; CI logs become noisy. Worse, scripts piping `--version` or capturing
help text get an unexpected stderr stream.

**Fix:** Print the banner *only after* parsing succeeds and the user explicitly invokes
either no subcommand or `Cmd::Version`. Keep the runtime mutual-exclusion check before
any benchmark work begins, but defer the banner.

### WR-08: `read_cpu_model` returns "unknown" on aarch64 Linux

**File:** `crates/alloc-bench-core/src/metrics/env.rs:33-42`

**Issue:** The Linux branch greps for `model name`, which only exists on x86 in
`/proc/cpuinfo`. On aarch64 (which the project explicitly supports per the stack doc:
*"aarch64-linux-* works"*), the file uses `Processor`, `CPU implementer`, `CPU part`
and similar — there is no `model name` line. Every aarch64-Linux Run record will have
`cpu_model: "unknown"`, which silently breaks the project's "environment-labelled"
constraint when comparing arm64 results.

**Fix:** Add a fallback chain:
```rust
for line in content.lines() {
    if let Some((k, v)) = line.split_once(':') {
        let k = k.trim();
        if matches!(k, "model name" | "Processor" | "CPU implementer") {
            return v.trim().to_string();
        }
    }
}
```
Even better: also try `/sys/devices/system/cpu/cpu0/of_node/compatible` for embedded
boards. Document the fallback order.

### WR-09: `parse_duration` ordering is fragile; `n * 60` for "m" suffix has no overflow guard

**File:** `crates/alloc-bench-cli/src/run.rs:14-30`

**Issue:** Two adjacent concerns:
1. **Suffix order is correctness-critical.** "ms" must be checked before "s"; if a
   maintainer reorders these branches, `"5ms"` would parse as `"5m" → 300s`. There is no
   test covering that ordering invariant.
2. **No overflow check.** `n * 60` (line 30) wraps silently for very large `n`. A user
   passing `--warmup 18446744073709551615m` produces a tiny `Duration` due to wrap. Not
   a security issue but a user-experience papercut.

**Fix:**
- Add a unit test asserting `parse_duration("5ms") == Duration::from_millis(5)` *and*
  `parse_duration("5m") == Duration::from_secs(300)` together (the test file has the
  former but only `2m`, so reordering "ms" and "s" would break the existing test —
  good — but adding the explicit `5m` and `5ms` adjacency catches the failure mode
  faster).
- Use `n.checked_mul(60).context("duration too large")?`.

## Info

### IN-01: `aggregator` placeholder exits with status 0 despite "not implemented" message

**File:** `crates/alloc-bench-aggregator/src/main.rs:1-3`

**Issue:** A `just bench-all` script that invokes the aggregator before Phase 4
will see a clean (zero) exit code despite the eprintln warning. Make failure visible:
```rust
fn main() -> std::process::ExitCode {
    eprintln!("alloc-bench-aggregator: not yet implemented (Phase 4 — see .planning/ROADMAP.md)");
    std::process::ExitCode::from(2)
}
```

### IN-02: `assert_mutual_exclusion` is dead code (already enforced by `compile_error!`)

**File:** `crates/alloc-bench-cli/src/allocator.rs:43-48`

**Issue:** The runtime check can never fire — if both features were enabled, the binary
would not compile. Keep the function as documentation of intent, but consider adding a
comment that explicitly says "unreachable; preserved for symmetry with the compile-time
check" or, more straightforwardly, delete it. The current "Defense-in-depth" comment is
slightly misleading since there is no second layer of defense to be in depth of.

### IN-03: `name()` and `stats().kind` can disagree (e.g., "mallocng" vs "system")

**File:** `crates/alloc-bench-cli/src/allocator.rs:21-40,52-72`

**Issue:** On musl with neither feature enabled, `name()` returns `"mallocng"` but
`stats()` returns `{"kind": "system"}`. The smoke test at
`tests/multithread_smoke.rs:47` asserts kind ∈ `{system, libmalloc, jemalloc, mimalloc}`
— it passes today because "mallocng" only ever appears as `name`, not `kind`. This is
intentional but the semantic split is undocumented and trips up downstream consumers.

**Fix:** Either align them (let `stats()` return `{"kind": "mallocng"}` etc.) or rename
the JSON field to `library_kind` to make clear it identifies the *library family*, not
the canonical allocator name.

### IN-04: `run_id` contains colons via RFC3339; problematic as a path component on some FSes

**File:** `crates/alloc-bench-cli/src/run.rs:72`

**Issue:** `run_id = format!("{}-{sha8}", chrono::Utc::now().to_rfc3339())` produces
something like `2026-05-18T12:34:56.789+00:00-12345678`. Colons are illegal in filenames
on Windows / FAT32 / some object-storage backends. If Phase 4 derives a directory name
from `run_id`, it will fail.

**Fix:** Use a filesystem-safe format, e.g.,
`chrono::Utc::now().format("%Y%m%dT%H%M%SZ").to_string()`.

### IN-05: `Run` and friends derive only `Serialize`; aggregator (Phase 4) will need `Deserialize`

**File:** `crates/alloc-bench-core/src/output.rs:5-87`

**Issue:** All schema structs derive `Serialize` but not `Deserialize`. Phase 4
aggregation will need to load JSON back into typed structs. Adding `Deserialize` now is
free and avoids a cross-cutting Phase-4 change.

**Fix:** Replace `#[derive(Debug, Serialize)]` with
`#[derive(Debug, Serialize, Deserialize)]` and import `serde::Deserialize`.

### IN-06: `BUILD_GIT_DIRTY` is a string `"true"`/`"false"` parsed in two places; should be a `bool` constant

**File:** `crates/alloc-bench-cli/build.rs:53-55`, `crates/alloc-bench-cli/src/build_info.rs:6`,
`crates/alloc-bench-cli/src/main.rs:55`, `crates/alloc-bench-cli/src/run.rs:86`

**Issue:** The build script writes the string "true" or "false" to an env var; the
runtime code does `build_info::GIT_DIRTY == "true"` in two places. Two consumers means
two opportunities for typo drift (e.g., capitalisation). Since all `env!` macros expose
strings, the cleanest pattern is a single helper:
```rust
// build_info.rs
pub fn git_dirty() -> bool { GIT_DIRTY == "true" }
```
…then call `build_info::git_dirty()` everywhere. Minor maintainability win.

---

_Reviewed: 2026-05-18_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
