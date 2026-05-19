---
phase: 02-scenario-fan-out
reviewed: 2026-05-18T00:00:00Z
depth: standard
files_reviewed: 14
files_reviewed_list:
  - crates/alloc-bench-cli/src/main.rs
  - crates/alloc-bench-cli/src/run.rs
  - crates/alloc-bench-cli/tests/run_all_smoke.rs
  - crates/alloc-bench-core/Cargo.toml
  - crates/alloc-bench-core/src/harness.rs
  - crates/alloc-bench-core/src/output.rs
  - crates/alloc-bench-core/src/scenarios/channels.rs
  - crates/alloc-bench-core/src/scenarios/contention.rs
  - crates/alloc-bench-core/src/scenarios/cpu_bound.rs
  - crates/alloc-bench-core/src/scenarios/fragmentation.rs
  - crates/alloc-bench-core/src/scenarios/mem_bound.rs
  - crates/alloc-bench-core/src/scenarios/mod.rs
  - crates/alloc-bench-core/src/scenarios/web.rs
  - scripts/dce_check.sh
findings:
  critical: 4
  warning: 11
  info: 6
  total: 21
status: findings
---

# Phase 2: Code Review Report

**Reviewed:** 2026-05-18
**Depth:** standard
**Files Reviewed:** 14
**Status:** findings

## Summary

Phase 2 implements 9 scenarios + a `run-all` registry on top of the Phase-1 harness. The structural approach is sound: scenario state lives behind `&mut self`, web/cpu-bound use scoped runtimes/pools, channels use `crossbeam-channel::bounded` with `std::thread::scope`, the schema additions (`Run.status`, `Run.error`, `ScenarioInfo.unit`) ride correctly on `skip_serializing_if = "Option::is_none"`, and the DCE script enforces a `__rust_alloc` floor. Test coverage is decent for unit-level smoke and one heavyweight `run_all_smoke` integration test.

However, **a release-profile / panic-strategy mismatch silently disables the entire `run-all` panic-isolation contract**, several scenarios have correctness gaps that will produce misleading numbers (mpsc/mpmc `allocations_per_tick`, fragmentation cap test flakiness, web tick payload determinism), and the DCE script has shell-pipeline failure-modes that mask build errors. There are also several Phase-1 footguns the new scenarios re-introduced (missing `validated()` on web/cpu-bound size validation against host RAM, missing panic propagation on web sub-task tokio errors, etc.). One bash defect would let a stale `.ll` from a previous run be inspected if cargo emits no IR for the current invocation.

The findings are listed below in severity order. CR-01 is the most consequential; it makes a documented failure-isolation promise unkeepable in the canonical `--release` build.

## Critical Issues

### CR-01: `panic = "abort"` in release profile breaks `panic::catch_unwind` in `run-all`

**File:** `Cargo.toml:39` (paired with `crates/alloc-bench-cli/src/run.rs:773-820` and `crates/alloc-bench-cli/tests/run_all_smoke.rs`)

**Issue:** The release profile sets `panic = "abort"`. With this setting, `std::panic::catch_unwind` is documented as **unable to catch panics — the process aborts directly** (per stdlib docs and the abort runtime, the unwinder is not even compiled in). The Phase-2 `run_all` design (`run.rs:766-820`) is built around catching per-scenario panics:

```rust
let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| -> Result<Run> {
    let mut scenario = builder()?;
    let outcome = run(&mut scenario, &cfg, allocator::stats)?;
    ...
}));
```

In a release build (the only build the project runs benchmarks under per CLAUDE.md "performance build flags"), if `Web::setup()` panics (e.g., port-bind failure on a host with `127.0.0.1` blocked), the entire `run-all` aborts the process and the remaining 9 scenarios are lost — exactly the failure mode CONTEXT.md says must NOT happen ("Continue on per-scenario failure"). The `run_all_smoke.rs` integration test exercises `Command::cargo_bin("alloc-bench-cli")` which runs the **release** binary by default (assert_cmd uses `cargo run` which respects the active profile, but in `cargo test` it's by default the dev profile — meaning the test passes because dev *uses unwind*, while CI/production run release where it aborts). The doc comment at `tests/run_all_smoke.rs:15-18` claims "this test runs against the release binary built by cargo", reinforcing the false safety.

This makes the `status: "failed"` + `error` recording path effectively dead code in the production build path. The schema-additive design and degenerate-failure-run helper are working code, but the `Err(panic) => ...` arm at `run.rs:815` will never execute in release mode.

**Fix:** Either (a) remove `panic = "abort"` from `[profile.release]` so `catch_unwind` is functional (the most consistent fix with the run-all contract), or (b) explicitly document that run-all panic-isolation is dev-only and run-all binaries should be built with a special profile that uses `panic = "unwind"`. Option (a) is recommended because it preserves the documented contract; the cost is slightly larger binary and slower panic codegen, both negligible for a benchmark tool.

```toml
# Cargo.toml — change line 39
[profile.release]
lto = "fat"
codegen-units = 1
opt-level = 3
strip = "symbols"
debug = false
# panic = "abort"        # REMOVED — run-all relies on catch_unwind
overflow-checks = false
```

If keeping `panic = "abort"` is intentional (e.g., for binary-size reasons), then add a separate profile (`[profile.bench-runall]`) with `panic = "unwind"` and require `cargo build --release --profile bench-runall` for run-all. Either way, the test must be re-run under the actual production profile to validate the contract.

### CR-02: `allocations_per_tick` for MPSC and MPMC overstates by up to `producers - 1` allocations

**File:** `crates/alloc-bench-core/src/scenarios/channels.rs:218-220, 301-303`

**Issue:** Both `Mpsc::allocations_per_tick` and `Mpmc::allocations_per_tick` return `self.cfg.objects_per_tick`. But the per-tick implementation distributes work via integer division (`per_producer = objects_per_tick / producers`, lines 229 and 308) — so the **actual** number of allocations performed is `producers * per_producer`, which equals `objects_per_tick - (objects_per_tick % producers)`. The comment at line 226-228 explicitly acknowledges the truncation but the metric does not reflect it:

```rust
let per_producer = cfg.objects_per_tick / cfg.producers as u64;
// ...
fn allocations_per_tick(&self) -> u64 {
    self.cfg.objects_per_tick     // wrong — overstates
}
```

For default `objects_per_tick=1000, producers=3` (a worked example from CLI defaults like 4 producers vs 1000 objects_per_tick gives `(1000/4)*4 = 1000` and is fine, but 1000/3 producers = 333*3 = 999, off by 1). For pathological `objects_per_tick=10, producers=3` only 9 sends happen but the metric reports 10 — a 10% overstatement that propagates through `metrics.allocations_per_tick * ticks_per_s` in the Phase-4 aggregator. Per WR-01 in the harness comments, "allocations_per_tick" is the documented dimension to derive allocator throughput from; an off-by-`producers-1` bias multiplied by the tick rate distorts the headline number.

The SPMC variant is fine because there's only one producer doing the full loop.

**Fix:** Compute the actual count produced:

```rust
fn allocations_per_tick(&self) -> u64 {
    let per_producer = self.cfg.objects_per_tick / self.cfg.producers as u64;
    per_producer.saturating_mul(self.cfg.producers as u64)
}
```

Or, alternatively, validate at config time that `objects_per_tick % producers == 0` and return `objects_per_tick`. Pick one and apply it to both `Mpsc` and `Mpmc`.

### CR-03: `dce_check.sh` line 49-54 — pipefail-tail combination silently swallows cargo failures and stale artifacts pass the gate

**File:** `scripts/dce_check.sh:43-73`

**Issue:** Two concurrent defects in the gate:

1. **Stale-artifact false-positive:** Line 43 (`rm -f target/release/deps/alloc_bench_cli-*.ll`) deletes prior `.ll` files, which is correct. But cargo will only re-emit `.ll` files if it actually re-builds the rustc step that produced them. If cargo's incremental cache already has the binary built **without** `--emit=llvm-ir` (e.g., from a prior `cargo build --release` step in the same CI job, which is very common given `just bench-all` style workflows), then `cargo rustc --release ... -- --emit=llvm-ir` is a no-op for the rustc invocation and **no `.ll` files are produced**. The script will `exit 1` with "no LLVM-IR files found" — but **only if no other** `.ll` was found at the glob (which, after `rm -f`, means it does fail loudly here).
   - However, the inverse is the real problem: if a prior run with a *different* feature flag (e.g., a prior `dce_check.sh jemalloc` and the user now runs `dce_check.sh system`) leaves leftover `.ll` files at slightly different paths (cargo deduplicates by hash of the build flags), the `rm -f target/release/deps/alloc_bench_cli-*.ll` only purges the matching CLI-binary IR. But if the binary was *just built* for the new flags and `--emit=llvm-ir` was a no-op (because cargo didn't re-run rustc), the old `.ll` from the previous flag's binary may remain, and the count gate passes against IR from the wrong allocator.
   - This is mitigated only by the fact that cargo's `--features` change forces a rebuild — but if anyone runs the script manually back-to-back with the same flag, the second run can pass-by-stale-IR.

2. **`tail -10` masks `--emit=llvm-ir` invocation errors:** Line 54 ends `2>&1 | tail -10`. With `set -o pipefail` (line 22), `pipefail` does propagate the rightmost non-zero exit code of the pipeline, so a cargo failure does cause the script to exit non-zero — that part is fine. But if cargo *succeeds* but rustc didn't emit IR (because the build was cached), the user only sees the last 10 lines of cargo's output, which typically reads `Finished release ...` and looks like success. The script then progresses to the grep stage and produces a mostly-irrelevant verdict.

   Additionally, the `tail -10` truncates the cargo error message itself when `cargo` does fail with a long error chain — the user sees only the trailing 10 lines, which often is unhelpful trailing diagnostic detail rather than the root error.

**Fix:**
- After cargo, explicitly verify `--emit=llvm-ir` was honoured by checking `find target/release/deps -name 'alloc_bench_cli-*.ll' -newer <stamp>`. Alternatively, `touch <stamp>` before cargo and `find ... -newer <stamp>` after.
- To force the rustc step to actually re-run when invoking `cargo rustc -- --emit=llvm-ir`, add `--target-dir target/dce-check` (separate target dir) or `cargo clean -p alloc-bench-cli` first. The current script's `rm -f` only purges output, not cargo's incremental fingerprint.
- Replace `2>&1 | tail -10` with a tee:
```bash
build_log=$(mktemp)
cargo rustc --release -p alloc-bench-cli "${FEATURE_FLAGS[@]}" --bin alloc-bench-cli \
    -- --emit=llvm-ir > >(tee "$build_log") 2>&1 || {
        echo "FAIL: cargo build failed" >&2
        cat "$build_log" >&2
        rm -f "$build_log"
        exit 1
    }
rm -f "$build_log"
```
or simpler, drop the `| tail -10` altogether and let cargo's full output stream out; CI logs handle truncation themselves.

### CR-04: Web scenario tick uses `cfg.seed` directly so payload is identical every tick (DCE / measurement risk)

**File:** `crates/alloc-bench-core/src/scenarios/web.rs:241-285`

**Issue:** At line 252, `tick()` constructs:

```rust
let payload = make_user_profile(&mut SmallRng::seed_from_u64(self.cfg.seed));
```

`self.cfg.seed` is a `const` field on the scenario — it never changes. This means **every tick generates the exact same `UserProfile`** (same `id`, same usernames, same metadata, same string lengths). Combined with `client.post().json(&payload).send()` repeatedly, the LLVM/HTTP layer above can in principle observe that the request body is identical request after request and:

1. Cache the serialised `Vec<u8>` of the payload — eliminating one of the *measured* serialisation allocations.
2. The reqwest connection-pool behavior uses identical bodies, so any per-payload allocation pattern is a single shape, not the heterogeneous distribution implied by `make_user_profile`'s `gen_range` calls (which evaluate identically every tick).
3. The handler's `p.id = p.id.wrapping_add(1)` defeats *whole-response* DCE, but the request side never varies.

The intent (per RESEARCH.md §Web scenario) is "Build a fresh payload (this is part of the alloc work we want to measure)" — but the seed never advances so the work is the same payload every tick. The PITFALLS.md / DCE discipline that the rest of the codebase carefully follows (mid-buffer write, `black_box`, etc.) is undercut here.

This is also a determinism trap: scenario authors elsewhere take `cfg.seed` and **derive** per-iteration RNGs (e.g., MPSC `cfg.seed.wrapping_add(p as u64)` at channels.rs:239). The web scenario's pattern reads as a copy of that idiom without the variance.

**Fix:** Either (a) hold a `SmallRng` in `self` (mirroring `FragmentationSoak::rng`, line 56-57) and let it advance across ticks, or (b) seed per-tick from a counter:

```rust
pub struct Web {
    cfg: WebConfig,
    runtime: Option<tokio::runtime::Runtime>,
    server_addr: Option<SocketAddr>,
    client: Option<reqwest::Client>,
    tick_seq: u64,    // monotonically increasing per tick
}

// in tick():
let seed = self.cfg.seed.wrapping_add(self.tick_seq);
self.tick_seq = self.tick_seq.wrapping_add(1);
let payload = make_user_profile(&mut SmallRng::seed_from_u64(seed));
```

Option (a) is preferable because it matches `FragmentationSoak`. Either way, the same fix applies to `cpu_bound.rs` if the input is intentionally re-cloned each tick (it is — line 144) but the cpu_bound `input` was randomised once at setup and is *deliberately* sorted from the same baseline each tick (you want comparable sort work). For web, varying the payload **is** part of the workload.

## Warnings

### WR-01: Empty `setup()` in `Box<dyn Scenario>` delegate vs harness invariant for web/cpu-bound

**File:** `crates/alloc-bench-core/src/harness.rs:34-53`

**Issue:** The `impl Scenario for Box<dyn Scenario>` delegate forwards `setup()` to the inner box: `(**self).setup()`. `harness::run` calls `scenario.setup()?` once before warmup. When `scenario` is a `&mut Box<dyn Scenario>`, the delegate is hit and the inner scenario's setup runs.

That works in normal flow. But the run-all closure builds the scenario *inside* `catch_unwind`, then calls `run(&mut scenario, ...)`. If the boxed scenario has expensive setup like `Web` (binds port, starts tokio runtime, builds reqwest client) and panics during setup (e.g., port-bind contention), the panic happens before any throughput is measured — which is fine for the per-scenario degenerate-failure record. **But** in release with `panic = "abort"` (CR-01), the process aborts. Even if CR-01 is fixed, if `Web::setup` panics partway through (e.g., after the runtime is built but before the client is built), the runtime drops in panic-unwind path and any tokio task already spawned is leaked or kicks an error message via `axum::serve`'s error path on `web.rs:218` — which is `eprintln!` in the unwind context.

**Fix:** Wrap the heavyweight `setup()` calls in their own tighter error mapping. For `Web::setup`:

```rust
fn setup(&mut self) -> anyhow::Result<()> {
    // Wrap the runtime build + bind + client build so partial state is cleared
    // on error and the next scenario in run-all starts clean.
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(self.cfg.server_workers)
        .enable_all()
        .build()
        .context("web scenario: build tokio runtime")?;
    // ... then if any sub-step errors, the partially-initialised state is
    // dropped here when we return early, and `self.runtime` stays None.
}
```

Currently the code uses `?` without `.context(...)` so the run-all error message is just the raw `std::io::Error` — `Address already in use (os error 98)` rather than `web scenario: bind 127.0.0.1:0: ...`. Hard to debug in the JSON output.

### WR-02: Web scenario error handling uses `.expect("...")` inside async tasks — these become panics, not errors

**File:** `crates/alloc-bench-core/src/scenarios/web.rs:266-269`

**Issue:**
```rust
let resp = client
    .post(&url)
    .json(&payload)
    .send()
    .await
    .expect("client.send failed");
resp.json::<UserProfile>()
    .await
    .expect("response.json failed")
```

A transient HTTP error (e.g., server slow to spawn, connection-reset because the server-side runtime is shutting down between ticks) becomes a panic inside `tokio::spawn`, which propagates up through `match h.await { Err(e) if e.is_panic() => std::panic::resume_unwind(e.into_panic()), ... }` (line 278) — taking down the entire scenario rather than recording a warning and continuing.

The harness has no notion of "tick that failed but other ticks succeeded" — `tick()` returning a panic kills the warmup loop or the measurement loop. In run-all this then converts to `status: "failed"` for the whole web scenario, even if 99% of ticks succeed.

**Fix:** Demote `.expect()` to error-propagation through `anyhow::Result`. But `tick()` returns `Box<dyn SinkValue>`, not a `Result`. The cleanest fix is to record the failure as a sentinel in the response and `black_box` it so it doesn't disappear, or to explicitly count failed ticks via a metric on `self`:

```rust
let resp = match client.post(&url).json(&payload).send().await {
    Ok(r) => r,
    Err(e) => {
        eprintln!("web tick: send failed (recorded as 0-byte response): {e}");
        return UserProfile { /* zero-value sentinel */ };
    }
};
```

Cleaner still: wrap individual tick failures in `Option<UserProfile>` and let the harness see `Vec<Option<UserProfile>>`. Either way, the present behaviour ("any HTTP error kills the whole scenario") is too brittle for a benchmark tool used for cross-allocator comparisons.

### WR-03: `cpu_bound.rs:148` uses `pool.install` but the merge step's `rayon::join` may still escape to the global pool

**File:** `crates/alloc-bench-core/src/scenarios/cpu_bound.rs:140-152`

**Issue:** The doc comment at line 1-15 explicitly states the goal is to use a scoped `rayon::ThreadPool` so the global pool is never touched. The implementation builds a scoped `pool` in `setup()` (line 116-118) and calls `pool.install(|| parallel_merge_sort(&mut data))` in `tick()` (line 148). However, `rayon::join` inside `parallel_merge_sort` (line 86) **uses the current thread's pool when called from inside `pool.install`**. That's correct for the top-level call.

But the recursive call `rayon::join(|| parallel_merge_sort(left), || parallel_merge_sort(right))` propagates correctly because rayon checks the current thread's registry. **However**, the `slice.sort_unstable()` base-case fallback at line 80 uses no rayon — that's fine. The actual subtle issue is that `rayon::join` is only thread-pool-aware when invoked from a worker thread of a pool. The top `pool.install(|| { rayon::join(...) })` correctly puts the calling thread into the pool. After install returns, recursion continues on rayon worker threads, which know their pool.

Re-reading the code: this part is **actually correct**. The doc comment correctly identifies why it works. **However**, there's a related issue: the cpu-bound scenario builds a fresh ThreadPool in `setup()` once (correct). But in `run-all`, multiple scenarios run in sequence; the cpu-bound pool is created on each scenario's `setup()` call. If `run-all` ever ran cpu-bound twice in a row (it doesn't), the pool would leak. This is more of a forward-looking concern.

The downgrade from CR to WR is because the actual implementation does honour the scoped-pool invariant for the cpu-bound scenario alone. The Warning is that **the pool field is `Option<rayon::ThreadPool>` and `teardown()` does not reset it** (the trait's default `teardown` is empty, line 47 of harness.rs). For a one-shot scenario this is fine, but if this scenario's struct is reused across multiple `run()` invocations (e.g., a future test harness) the second `setup()` would attempt to overwrite a still-running pool. Add explicit `teardown()` that drops the pool:

```rust
fn teardown(&mut self) {
    self.pool.take();
    self.input.take();
}
```

### WR-04: `cpu_bound.rs:121` shuffle loop will allocate ~`input_size_mb * 1024 * 1024 / 8` element vec in setup — for `input_size_mb=64MB`, that is 8M u64 = 64MB. For run-all default `input_size_mb=2`, OK; for CLI default 64, this allocates 64MB once per `setup()`

**File:** `crates/alloc-bench-core/src/scenarios/cpu_bound.rs:119-126`

**Issue:** Not a bug per se but worth a defensive bound. In `tick()` (line 144), the input is *cloned* every tick. With CLI default `input_size_mb=64` and a 60s `--duration`, expect ~10-30 sorts/s × 64 MB clone × duration = a steady 64 MB tick allocation in addition to merge-step allocations. That's the design (you *want* the clone to be measured), but `validated()` does not reject pathologically large `input_size_mb` (e.g., 1_000_000 would attempt to allocate 1 TB and crash the host's OOM killer rather than gracefully erroring).

**Fix:** Add a sanity guard on `input_size_mb`:

```rust
pub fn validated(self) -> anyhow::Result<Self> {
    anyhow::ensure!(self.threads >= 1, "threads must be >= 1 (got {})", self.threads);
    anyhow::ensure!(
        self.input_size_mb >= 1 && self.input_size_mb <= 4096,
        "input_size_mb must be in [1, 4096] (got {})", self.input_size_mb
    );
    Ok(self)
}
```

Same applies to `mem_bound.rs:48` (size_mb has only `>= 1`) and `realloc_storm.rs:32-41` (target_size_mb has only `>= 1` with comment "left to the user"). The realloc-storm `>= 1` allows the user to push 16 EiB which never errors but pushes 16 EiB worth of `Vec::push` calls — an infinite loop in practice.

### WR-05: `realloc_storm.rs:73-74` overflow — `(target_size_mb as u64) * 1024 * 1024` can overflow `usize` on 32-bit

**File:** `crates/alloc-bench-core/src/scenarios/realloc_storm.rs:73, 78`

**Issue:** Line 78: `let target_bytes = self.cfg.target_size_mb * 1024 * 1024;` — `target_size_mb: usize`. On 64-bit, an attacker passing `target_size_mb = usize::MAX / 1024 / 1024` (i.e., ~16 PiB) wraps the multiply silently because `[profile.release]` has `overflow-checks = false` (Cargo.toml:40). The `for i in 0..target_bytes` loop then iterates a wrapped-small number of times and reports nominal success — exactly the kind of silent wrap WR-09 in `parse_duration` was hardened against.

Line 73 inside `allocations_per_tick`: `(self.cfg.target_size_mb as u64) * 1024 * 1024` — the cast to `u64` first means this can't overflow on 64-bit, but the inputs come from a `usize` cast that was already validated as `>= 1`. This computation is fine on the hot path **only** because the validated bound is `>= 1`; the upper bound is unchecked.

**Fix:** Add `checked_mul`-style validation in `validated()`:

```rust
pub fn validated(self) -> anyhow::Result<Self> {
    anyhow::ensure!(
        self.target_size_mb >= 1,
        "target_size_mb must be >= 1 (got {})", self.target_size_mb
    );
    let _ = (self.target_size_mb as u64)
        .checked_mul(1024 * 1024)
        .with_context(|| format!("target_size_mb={} overflows u64 byte count", self.target_size_mb))?;
    // also ensure it fits in usize on this platform:
    anyhow::ensure!(
        self.target_size_mb <= isize::MAX as usize / (1024 * 1024),
        "target_size_mb must be <= {} on this platform", isize::MAX as usize / (1024 * 1024)
    );
    Ok(self)
}
```

### WR-06: Channel scenarios `validated()` does not enforce `consumers == 1` for MPSC at config-construction (only at CLI level)

**File:** `crates/alloc-bench-cli/src/run.rs:255-259, crates/alloc-bench-core/src/scenarios/channels.rs:80-106`

**Issue:** The MPSC contract is "Multi-Producer Single-Consumer" — `consumers must equal 1`. The CLI dispatcher enforces this at `run.rs:255-258`:

```rust
ensure!(
    consumers == 1,
    "MPSC requires --consumers 1 (got {consumers})"
);
```

But `ChannelConfig::validated` accepts any `consumers >= 1`. So the registry path at `run.rs:578-590` constructs `Mpsc::new(ChannelConfig{producers:4, consumers:1, ...}.validated()?)` — enforced in the right place there. **But** if a future caller bypasses the CLI and calls `Mpsc::new(ChannelConfig{producers: 1, consumers: 5, ...}.validated().unwrap())`, the MPSC implementation runs with 5 consumers — silently degenerate (the single-receiver code path at line 254 only reads from one of the cloned receivers, but actually creates only one because it's `let r = ...` not a clone-loop). So 4 of those 5 consumers do not exist; the metric overstates.

Wait — re-reading channels.rs:222-272: the MPSC tick creates 1 receiver in this thread (`while let Ok(msg) = r.recv()`), regardless of `cfg.consumers`. So passing `consumers=5` is silently ignored — `cfg.consumers` is a serialised field but not used for receivers. The `Mpsc` validation is missing the topology constraint.

Similarly, `Spmc::tick` uses `cfg.consumers` for the loop, so SPMC-with-`producers=5` is a *silent error* — only 1 producer runs (this thread, line 165), but `cfg.producers=5` is serialised into `config_json` as if 5 producers ran. Future plotting will show 5x throughput per producer when actually 1 producer did all the work.

**Fix:** Move topology constraints into `validated()`:

```rust
impl ChannelConfig {
    pub fn validated_for(self, kind: ChannelKind) -> anyhow::Result<Self> {
        // basic >=1 checks (current code) ...
        match kind {
            ChannelKind::Spmc => anyhow::ensure!(self.producers == 1, ...),
            ChannelKind::Mpsc => anyhow::ensure!(self.consumers == 1, ...),
            ChannelKind::Mpmc => {} // both >= 1 sufficient
        }
        Ok(self)
    }
}
```

Then call `.validated_for(ChannelKind::Mpsc)` from the CLI and registry alike. Or split into `SpmcConfig`/`MpscConfig`/`MpmcConfig` newtypes that wrap `ChannelConfig` and enforce the topology at the type level (more code, more safety).

### WR-07: `panic_message` clippy lint — `&Box<dyn Any + Send>` should be `&dyn Any` (Box-deref)

**File:** `crates/alloc-bench-cli/src/run.rs:710`

**Issue:** `fn panic_message(payload: &Box<dyn std::any::Any + Send>) -> String` triggers `clippy::borrowed_box` — `&Box<T>` is almost always a code smell because `&dyn T` does the same job. More substantively, `payload.downcast_ref::<&'static str>()` works on the `Box`'s deref but you could call it directly on `&dyn Any`. Not a correctness defect; a maintainability one.

**Fix:**
```rust
fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(s) = payload.downcast_ref::<&'static str>() {
        ...
    }
}
// caller: panic_message(&*panic)  // or panic_message(panic.as_ref())
```

### WR-08: `cpu_bound.rs:137` allocations_per_tick formula is misleading — it's "elements", not "allocations"

**File:** `crates/alloc-bench-core/src/scenarios/cpu_bound.rs:132-138`

**Issue:** The comment at line 132-136 acknowledges the metric is approximate — total alloc nodes ~= ceil(log2(elems)) per recursive sort, but the implementation returns the element count (line 137-138). For `input_size_mb=64`, that's 8M — but the real allocation count per tick is `2 * (log2(8M) - 10)` ≈ 26 merge-step allocs + 1 input clone = 27 allocations, off by **300_000x**. The aggregator then computes `ticks_per_s * allocations_per_tick` and prints "allocations/s" that is wildly wrong.

The Phase-4 aggregator can't recover from a 5-decimal-order bias by post-processing. Other scenarios use accurate counts:
- `multithread`: `threads * objects` ✓ accurate
- `contention`: `threads * iters_per_tick` ✓ accurate (alloc + drop is one alloc-pair per iter)
- `mem_bound::LinkedList`: `size_mb * 1024 * 1024 / 64` ✓ accurate
- `realloc_storm`: `log2(target_bytes)` ✓ approximate but realistic
- `fragmentation`: `allocs_per_tick` ✓ accurate
- `web`: `client_workers` ✓ approximate
- `cpu_bound`: `input_size_mb * 1024 * 1024 / 8` ✗ this is the **element** count, not the alloc count

**Fix:**
```rust
fn allocations_per_tick(&self) -> u64 {
    let n_elems = (self.cfg.input_size_mb as u64) * 1024 * 1024 / 8;
    // Merge-sort recursion: each level merges into a fresh Vec, and there are
    // ceil(log2(n_elems / 1024)) levels above the base-case cutoff (line 80).
    // Plus 1 for the per-tick input clone at line 144.
    let levels = (n_elems / 1024).next_power_of_two().trailing_zeros() as u64;
    // 2^levels - 1 internal nodes, each does one alloc; +1 for input clone.
    (1u64 << levels).saturating_sub(1).saturating_add(1)
}
```

Or, more pragmatically: document the metric as "approximate" in the schema field name and stop trying to derive `allocs/s` for cpu-bound from it. The current value is misleading enough to warrant a fix even in the absence of a precise model.

### WR-09: `harness.rs:118` `samples_count` is `hist.len()` which counts samples but the divisor `measurement_s` may be 0.0 if every tick exceeded HIST_MAX_NS

**File:** `crates/alloc-bench-core/src/harness.rs:117-121`

**Issue:**
```rust
let measurement_s = measure_start.elapsed().as_secs_f64();
let samples_count = hist.len();
let ticks_per_s = samples_count as f64 / measurement_s;
```

If `cfg.measure` is `Duration::from_secs(0)` (the harness rejects warmup<1s but does not validate `measure`), the while-loop runs zero iterations, `samples_count=0`, `measurement_s` is approximately zero, and `ticks_per_s = 0.0 / 0.0 = NaN`. NaN serialises to `null` in JSON in serde_json (or panics with `serde_json::Error::custom("invalid float value: NaN")` depending on the float encoder). Additionally, the Phase-4 aggregator that does `ticks * allocations` produces NaN.

The phase-4 schema doesn't define how NaN is handled; but this is a degenerate config that should error.

**Fix:** Add `cfg.measure >= Duration::from_secs(1)` (or `> Duration::ZERO`) check at the top of `harness::run`:

```rust
if cfg.warmup < Duration::from_secs(1) {
    bail!("warm-up must be >= 1s; ...");
}
if cfg.measure < Duration::from_secs(1) {
    bail!("measure must be >= 1s; otherwise ticks/s is undefined");
}
```

### WR-10: `output.rs:1` — `pub const SCHEMA_VERSION: u32 = 1` but Phase-2 made schema additions; the additive contract is enforceable only via tests, not at the type level

**File:** `crates/alloc-bench-core/src/output.rs:1-25`

**Issue:** Phase-2 added 3 fields (`Run.status`, `Run.error`, `ScenarioInfo.unit`) all behind `skip_serializing_if = "Option::is_none"` — meaning Phase-1 byte-equivalence holds. Two unit tests (lines 109-244) validate this. Good. **However**, a future Phase-3 could add a *required* field (no `skip_serializing_if`) and the SCHEMA_VERSION constant would not get bumped automatically. There is no machine-checked link between "I added a non-additive field" and "I bumped SCHEMA_VERSION".

**Fix:** Add a `cargo expand`-style snapshot test of the JSON shape for a known-fixed Run, asserting both the Phase-1 byte-equivalence (already done at lines 109-177) and a Phase-2-specific shape (asserting the presence of the optional fields when populated). When someone adds a new field, the snapshot fails and forces them to either update the test (bumping SCHEMA_VERSION in the same change) or revert the field. Lower-tech alternative: a `// SCHEMA_VERSION:1` doc-comment that an `xtask` greps for.

### WR-11: `fragmentation.rs:184-189` test asserts cap is *exactly* hit, which is probabilistic and may flake

**File:** `crates/alloc-bench-core/src/scenarios/fragmentation.rs:182-189`

**Issue:**
```rust
assert!(
    s.long_lived_len() == 100,
    "expected long_lived to reach cap of 100, got {}",
    s.long_lived_len()
);
```

The test runs 50 ticks of `allocs_per_tick=10_000` (so ~1000 long-lived attempts per tick × 50 = 50_000 attempts). The cap is 100. After it's hit, every push *replaces* (swap_remove + push), so `len == 100` should be stable. Re-reading the implementation (line 115-119):

```rust
if self.long_lived.len() >= self.cfg.long_lived_cap {
    let evict_idx = self.rng.gen_range(0..self.long_lived.len());
    self.long_lived.swap_remove(evict_idx);
}
self.long_lived.push(b);
```

Once `len >= cap`, every iteration: evict 1, push 1, net 0. So len is strictly `cap` after first hit. This holds **only if** at least one long-lived push happens before the assertion. With probability `1 - 0.9^(10_000)` per tick that long-lived hits cap, the test should hit cap in tick 1 (probability 1 - tiny epsilon). The comment correctly notes "negligible" probability — but is it actually negligible? `0.9^10_000 ≈ 1e-458`, so yes negligible.

**However**, the SmallRng with seed=1 is deterministic. The test is fine in practice. The only flakiness risk is if `SmallRng`'s implementation changes between Rust versions and the seeding produces a degenerate sequence. That's a stability-of-the-PRNG concern. Mark as Warning rather than Critical because the test will catch a regression in 99.9999...% of runs.

**Fix:** Make the assertion robust to the rng implementation:

```rust
// After ticks, len() should be exactly the cap unless RNG gave 0 long-lived
// pushes ever (probability ≪ 1e-50 with these knobs).
assert_eq!(
    s.long_lived_len(),
    100,
    "expected long_lived to reach cap of 100; if not, RNG sequence is degenerate (regenerate with a different seed)"
);
```

Or run more ticks with a smaller cap to make the convergence even faster. (Current is fine; just document the assumption better.)

## Info

### IN-01: `main.rs:222` and `run.rs:79` duplicate sha-truncation logic — extract to a helper

**File:** `crates/alloc-bench-cli/src/main.rs:222`, `crates/alloc-bench-cli/src/run.rs:79`

**Issue:** Both `main.rs:222` and `run.rs:79` compute:

```rust
let sha = build_info::GIT_SHA;
let sha8 = &sha[..sha.len().min(8)];
```

The `.min(8)` guard is correct (handles a sha shorter than 8 chars — empty string from a shallow checkout, e.g.). Two copies in two files. If the truncation policy ever changes (e.g., to 12 chars), both must be updated.

**Fix:** Add `pub fn short_sha() -> &'static str` to `build_info` (or to a `util` module).

### IN-02: `run.rs:519-705` `default_scenarios` is 187 lines of boilerplate; could shrink with a macro

**File:** `crates/alloc-bench-cli/src/run.rs:519-705`

**Issue:** Each entry in `default_scenarios` is structurally identical: name string, optional unit, builder closure that constructs a `Config { ... }.validated()?` then `ScenarioType::new(cfg)`, with each field hardcoded. A `macro_rules!` would tighten this to ~80 lines and make adding the next scenario one line.

**Fix:** Optional refactor; not a defect. Suggest a follow-up `gsd-quick` task.

### IN-03: `web.rs:131-132` hardcoded RFC3339 dates in payload

**File:** `crates/alloc-bench-core/src/scenarios/web.rs:131-132`

**Issue:**
```rust
let created_at = "2026-05-18T10:00:00Z".to_string();
let last_login = "2026-05-18T11:00:00Z".to_string();
```

These are deterministic, which is the design intent. But hardcoded date matches `#currentDate 2026-05-18` from CLAUDE.md — a maintainer reading this in 2027 might think this should be live. Add a comment or use a constant.

**Fix:**
```rust
// Hardcoded: payload determinism is more important than fresh timestamps for
// allocator stress benchmarking. Both fields are 20 ASCII bytes so the
// resulting JSON layout is invariant.
const CREATED_AT: &str = "2026-05-18T10:00:00Z";
const LAST_LOGIN: &str = "2026-05-18T11:00:00Z";
```

### IN-04: `run.rs:533-538` reimports inside function shadow the module-level `use` (`use alloc_bench_core::scenarios::{...}` at line 5-10)

**File:** `crates/alloc-bench-cli/src/run.rs:5-10, 533-538`

**Issue:** The module-level `use alloc_bench_core::scenarios::{ChannelConfig, Contention, ...}` (line 5-10) already imports everything `default_scenarios` needs. The `use` inside `default_scenarios` (line 533-538) is redundant — the same names are imported twice and clippy will not flag this because it's inside a different scope. Doesn't cause a bug; reads as forgotten cleanup.

**Fix:** Delete line 533-538.

### IN-05: `cpu_bound.rs:189` — base-case cutoff `1024` is a magic number

**File:** `crates/alloc-bench-core/src/scenarios/cpu_bound.rs:77-82`

**Issue:**
```rust
if slice.len() <= 1024 {
    slice.sort_unstable();
    return;
}
```

`1024` appears as a hardcoded cutoff. Phase-4 aggregator may compare cpu-bound results across allocators and the cutoff biases the alloc-vs-no-alloc ratio. Document or constify.

**Fix:**
```rust
const BASE_CASE_CUTOFF: usize = 1024;  // below this, std unstable sort (no recursion / no alloc)
```

### IN-06: `dce_check.sh:63` shopt nullglob/unset is correct but the array reference at line 73 (`"${LL_FILES[@]}"`) breaks if `LL_FILES` is empty (already guarded at line 64, OK)

**File:** `scripts/dce_check.sh:60-73`

**Issue:** The flow is correct: `nullglob` makes the glob expand to nothing on no-match, length-0 check at 64 catches that, only then does line 73 reference `"${LL_FILES[@]}"`. But on bash <4.0 (macOS default `/bin/bash` is 3.2.x — see `bash --version`), `nullglob` works but combined with `set -u` (line 22), referencing `${LL_FILES[@]}` on an empty array can error. The `[ ${#LL_FILES[@]} -eq 0 ]` check itself is safe under `set -u`.

**Fix:** Test the script on macOS bash 3.2 (the common dev env per CLAUDE.md "macOS host"). Or hint at the shebang's bash 4+ requirement; or rewrite to avoid the array entirely. Low risk because `set -e` would catch the failure loudly.

---

_Reviewed: 2026-05-18_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
