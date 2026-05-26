# FEATURES.md — Allocator Benchmark Feature Set

## Categorization

**Table stakes** — must have for the benchmark to be credible.
**Differentiators** — make this benchmark stand out vs ad-hoc comparisons.
**Anti-features** — common mistakes that bias results, deliberately not built.

## Benchmark Scenarios

### Table stakes (must have)

| Scenario | Why it matters |
|----------|----------------|
| **Multi-thread allocation stress** | Spawn N threads each allocating M objects of mixed sizes — directly exposes per-thread arena scaling and lock contention. Closest to "what users mean" when they say "allocator performance". |
| **Web-service ser/de** | Most realistic single workload: tokio runtime + serde + small/medium heap objects per request. Generates a wide allocation-size distribution at high concurrency. |
| **MPSC channels** | Producer threads `Box::new()` payloads, single consumer drains. Tests cross-thread free → re-alloc pattern (where most allocators struggle differently). |
| **MPMC channels** | Same but multi-consumer: stress allocator's cross-thread free behavior plus contention. |
| **SPMC channels** | One producer, many consumers fan-out: tests producer-side allocation throughput. |
| **CPU-bound algorithm** | Parallel merge-sort or matrix-multiply with allocations in the critical path — measures allocator overhead in compute workloads. |
| **Memory-bound algorithm** | Pointer-chasing linked-list with random insertion — stresses allocator's locality (cache-friendly placement) and fragmentation. |
| **Lock-contention / arena saturation** | High thread count (≥ CPU count × 2) with rapid alloc/free of same size — distinguishes thread-cached allocators (jemalloc, mimalloc) from contention-bound ones (ptmalloc). |

### Differentiators (suggested additions beyond user's PLAN.md)

| Scenario | Why it adds value |
|----------|-------------------|
| **Fragmentation soak test** | Long-running mixed alloc/free with biased sizes — measures resident-vs-allocated divergence over time. Where allocators differ most in real production. |
| **Short-lived vs long-lived mix** | 80% objects freed within 1ms, 20% live for the full run — this is the actual production heap shape. Few benchmarks capture it. |
| **Cold-start latency** | Time from `main()` to first allocation, then time to steady-state. mimalloc and jemalloc have lazy init; ptmalloc is eager. Affects serverless / CLI workloads. |
| **Producer/consumer payload-size variance** | Channel benches with payload size drawn from a Pareto distribution, not uniform — closer to real systems. |
| **Realloc storm** | `Vec::push()` under pressure — many allocators have fast paths for grow-in-place that diverge dramatically. |

### Anti-features (deliberately not built)

| Anti-feature | Why avoid |
|--------------|-----------|
| **Single-threaded micro-allocation loop** | Trivial; doesn't expose any inter-allocator difference; misleads readers into thinking allocators are equivalent. |
| **Synthetic random alloc/free with no temporal pattern** | Production heaps are not uniform random; uniform random gives all allocators near-equal scores. |
| **Benchmarks with allocations optimized away by the compiler** | Black-box wrapping is mandatory (see PITFALLS.md). |
| **Mean-only reporting (no percentiles)** | p50 hides tail-latency divergence which is exactly where allocators differ. Always report p50/p95/p99/p999. |
| **Per-iteration setup in measurement loop** | Setup must be in warm-up; measurement is steady-state only. |

## Metrics

### Table stakes

| Metric | Granularity | Source |
|--------|-------------|--------|
| Throughput (ops/sec) | per-bench, per-allocator, per-env | wall-clock + counter |
| Latency p50/p95/p99/p999 | per-op | hdrhistogram |
| Peak RSS | per-run | getrusage(RUSAGE_SELF).ru_maxrss |
| RSS growth curve | sampled every 1s | /proc/self/statm |
| Wall-clock duration | per-run | std::time::Instant |
| Page faults (major/minor) | per-run | getrusage |

### Differentiators

| Metric | Granularity | Source |
|--------|-------------|--------|
| Allocator-internal stats | per-run | tikv_jemalloc_ctl::stats, mimalloc mi_stats_print |
| Resident vs allocated ratio | per-run | jemalloc: stats::resident / stats::allocated. Proxy for fragmentation. |
| Time-to-first-allocation | startup | timestamp before/after first heap touch |
| CPU time (user/sys) | per-run | getrusage |
| Voluntary context switches | per-run | getrusage |

## Canonical reference benchmarks

The mimalloc paper (Microsoft Research, 2019) and the jemalloc paper benchmark against:

- **mimalloc-bench** suite: cfrac, espresso, redis-bench, glibc-thread, larson, mstress, sh6bench, xmalloc-test, malloc-large, etc.
- **shbench** — Synthetic Heap Benchmark from MicroQuill — small/medium objects, threaded.
- **threadtest** — Hoard suite, multi-thread alloc/free.
- **larson** — Server simulation, long-lived objects.

We won't port these (license / time), but our scenarios cover the same axes:
- larson ≈ web-service bench
- threadtest ≈ multi-thread allocation stress
- mstress ≈ MPMC channels
- malloc-large ≈ memory-bound bench

This gives our results comparability to published allocator papers without re-implementing C suites.

## Warm-up patterns

| Allocator | Lazy init? | Recommended warm-up |
|-----------|-----------|---------------------|
| ptmalloc (glibc) | Mostly eager; arenas created lazily on first contention | 1–2s |
| mallocng (musl) | Eager | 1s |
| jemalloc | Lazy: arenas + tcache populated on demand | 5s — significant |
| mimalloc | Lazy: segment cache, page cache | 5s — significant |

**Rule:** warm-up duration = max(5s, 10% of measurement duration). Always do warm-up regardless of allocator (cold-start results have high variance).

## Anti-pattern: warm-up too short

Reported in mimalloc paper: < 1s warm-up causes mimalloc and jemalloc to look ~2x worse than steady-state. ptmalloc is more honest with short warm-ups, but that's its weakness (no caching to warm up), not a fairness signal.

## Web-bench request shape

Recommendation: nested JSON struct, ~1.5 KB request body / ~1.5 KB response body. Mix of strings, arrays, and numbers. Drives both serde_json deserialization (many small `String` allocations) and Vec growth (response payload). High allocation count per request without dominating CPU.

```rust
#[derive(Deserialize)]
struct Request {
    user_id: String,
    items: Vec<Item>,        // 5-20 items
    metadata: HashMap<String, Value>, // 3-10 keys
}
#[derive(Serialize)]
struct Response {
    request_id: String,
    items: Vec<EnrichedItem>,
    timing: TimingInfo,
}
```

## Channel-bench payload shape

`Box<Payload>` where `Payload` contains a `Vec<u8>` of size drawn from a distribution. Key design: payloads are heap-allocated on the producer side, deallocated on the consumer side — this cross-thread free is precisely where ptmalloc and the modern allocators diverge most.

Distributions to support via CLI:
- uniform [16, 1024]
- bimodal: 90% small (16 B), 10% large (4 KiB)
- pareto (heavy-tailed)

## Memory-bound bench design

Two sub-scenarios (CLI selectable):

1. **Pointer-chasing linked list:** insert N nodes, randomly shuffle, traverse. Tests allocator's spatial locality — well-placed allocations have linked-list traversal hitting cache; poor locality crushes performance.
2. **Large strided array:** allocate one giant `Vec<u64>`, traverse with stride > cache line. Stresses memory subsystem, not allocator. Useful as a control: confirms allocator isn't the bottleneck for this workload.

Both expose different signal. Recommendation: run both; the linked-list result is the meaningful allocator metric, the array result is the control.

## Lock-contention / arena bench

Spawn `2 × num_cpus()` threads. Each does tight loop: `Box::new([0u8; 64])` → use → drop. No payload distribution — all same size to maximize arena collision.

Expected ranking (priori): mimalloc ≈ jemalloc ≫ mallocng > ptmalloc, with the gap growing with thread count. This is where the allocator ranking is most stark.

## Output format requirement

Every scenario emits a single results record with:
- `scenario_name`, `scenario_config` (params)
- `env` (os, kernel, docker_image, cpu_model)
- `build` (allocator_name, rustc_version, target_triple, profile, timestamp, git_sha)
- `metrics` (throughput, latencies dict, peak_rss_kb, rss_growth_samples, alloc_stats dict, rusage dict)
- `samples_count` and `warmup_duration_s` and `measurement_duration_s` for reproducibility

This schema is the input to the Plotly aggregator.
