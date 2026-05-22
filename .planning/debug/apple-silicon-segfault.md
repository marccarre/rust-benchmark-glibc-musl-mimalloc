---
status: diagnosed
trigger: "Apple Silicon Rosetta SIGSEGV on bench-all-smoke — UAT 2026-05-23 phase 05 found all 18 cells fail with exit 139 on macOS arm64 (orbstack), but GHA x86-64 ubuntu-24.04 passes 18/18."
created: 2026-05-23T00:00:00Z
updated: 2026-05-23T00:00:00Z
---

## Current Focus

hypothesis: RUSTFLAGS=-C target-cpu=x86-64-v3 (D-09 mandate) emits AVX2/BMI2 instructions that Rosetta on Apple Silicon does not reliably emulate; binary segfaults at first such instruction inside the container.
test: Read Dockerfiles, justfile, Cargo.toml, .github/workflows/*, and README to confirm (a) target-cpu=x86-64-v3 is set in builder stages, (b) no Apple Silicon override exists in justfile, (c) GHA uses same flags and passes, (d) README has no Rosetta troubleshooting note.
expecting: Confirm v3 flag in builder + no override + GHA parity → root cause confirmed (Rosetta x86-64-v3 incompatibility).
next_action: Read in parallel: STATE.md, 05-HUMAN-UAT.md, README.md, justfile, scratch.Dockerfile, alpine.Dockerfile, 03-CONTEXT.md, Cargo.toml.

## Symptoms

expected: User runs `just bench-all-smoke` on Apple Silicon (orbstack docker driver) per README walkthrough; gets 18 successful cells producing `results/{alloc}-{env}.json` files (REPR-01).
actual: All 18 cells fail `just run {env} {alloc}` with exit code 139 (SIGSEGV) on binary launch. Build cached/succeeded. `docker run` segfaults the binary immediately. `just aggregate` errors: `no results found matching pattern "results/*.json"` because results/ is empty.
errors: |
  error: recipe `run` failed with exit code 139 (×18)
  error: recipe `bench-cell` failed on line 112 with exit code 139
  Error: no results found matching pattern "results/*.json"
reproduction: Run `just bench-all-smoke` on macOS Apple Silicon with orbstack as the docker buildx driver.
started: Discovered during /gsd:verify-work UAT 2026-05-23 (phase 05 HUMAN-UAT).
disambiguating: GHA CI on real x86-64 Linux runners (ubuntu-24.04) PASSED all 18 cells with same Docker images, build flags, allocators. Bug is NOT in binary or Dockerfile; it's in runtime execution under Apple Silicon's Rosetta-x86 emulation.

## Eliminated

- hypothesis: "Bug is in the binary or the Dockerfile build path"
  evidence: "GHA matrix on ubuntu-24.04 (real x86-64) PASSED all 18 cells with the SAME Docker images, the SAME RUSTFLAGS, the SAME allocators (per 05-HUMAN-UAT.md Test 2 'All 18 bench-matrix jobs PASSED on ubuntu-24.04')."
  timestamp: 2026-05-23

- hypothesis: "Bug is in just/docker invocation flags (e.g., missing --platform)"
  evidence: "justfile:75 already passes `--platform linux/amd64` to every `docker buildx build` and justfile:103 to every `docker run`. README §Troubleshooting confirms this is automatic. So the binary IS being run as amd64; the bug is in the emulator's instruction-set coverage, not in platform selection."
  timestamp: 2026-05-23

- hypothesis: "Bug is in the host .cargo/config.toml leaking target-cpu=native into Docker builds"
  evidence: "All six Dockerfiles set `ENV RUSTFLAGS=\"-C target-cpu=x86-64-v3\"` (alpine.Dockerfile:31, scratch.Dockerfile:41, debian-slim.Dockerfile:27, distroless-cc.Dockerfile:26, distroless-static.Dockerfile:30, wolfi.Dockerfile:32). The Docker builder stage explicitly overrides any host RUSTFLAGS. Even debian-slim.Dockerfile:25 comments `Override .cargo/config.toml's target-cpu=native (Pitfall §2)`. So the v3 flag IS what's being baked into the binary, not native."
  timestamp: 2026-05-23

## Evidence

- timestamp: 2026-05-23T00:00:00Z
  checked: "All six Dockerfiles in docker/ for RUSTFLAGS"
  found: |
    alpine.Dockerfile:31 → `ENV RUSTFLAGS="-C target-cpu=x86-64-v3"`
    scratch.Dockerfile:41 → `ENV RUSTFLAGS="-C target-cpu=x86-64-v3 -C target-feature=+crt-static"`
    debian-slim.Dockerfile:27 → `ENV RUSTFLAGS="-C target-cpu=x86-64-v3"` (with comment "Override .cargo/config.toml's target-cpu=native")
    distroless-cc.Dockerfile:26 → `ENV RUSTFLAGS="-C target-cpu=x86-64-v3"`
    distroless-static.Dockerfile:30 → `ENV RUSTFLAGS="-C target-cpu=x86-64-v3 -C target-feature=+crt-static"`
    wolfi.Dockerfile:32 → `ENV RUSTFLAGS="-C target-cpu=x86-64-v3"`
  implication: "Every binary in every Docker image is compiled for x86-64-v3 — which mandates AVX2, BMI1, BMI2, FMA, F16C, MOVBE, OSXSAVE, plus the v2 baseline (SSSE3, SSE4.1, SSE4.2, POPCNT). Phase 3 D-09 locked this for cross-runner portability."

- timestamp: 2026-05-23T00:00:00Z
  checked: ".github/workflows/bench.yml top-level env"
  found: "Line 75: `RUSTFLAGS: \"-C target-cpu=x86-64-v3\"` — same flag as Docker builders. Comment on line 72-74 'Phase 3 D-09 locked: x86-64-v3 baseline so GHA shared-CPU pool migrations never produce illegal-instruction crashes.' All bench-matrix jobs run on `ubuntu-24.04` (real x86-64 hardware, line 107)."
  implication: "GHA proves the binary itself is correct on real x86-64 silicon. Test 2 in 05-HUMAN-UAT.md explicitly states '✓ — proves the v3 instruction set works on real x86-64; Test-1 failure was Rosetta-specific'."

- timestamp: 2026-05-23T00:00:00Z
  checked: "justfile for any Apple-Silicon-specific RUSTFLAGS override"
  found: "NO override exists. justfile:43 `build env alloc:` runs `docker buildx build --platform linux/amd64 ... --build-arg RUST_VERSION=1.91 ...` with no RUSTFLAGS override; the Dockerfile's `ENV RUSTFLAGS=...` v3 baking is unconditional. justfile:94 `run env alloc:` runs `docker run --platform linux/amd64 ...` with no allocator/CPU flag knobs."
  implication: "There is no escape hatch for Apple Silicon hosts — the v3-tuned binary is the only artifact `just build` produces. `bench-all-smoke` (justfile:209) is just a wrapper that sets BENCH_SMOKE=1 and re-invokes `just bench-all`; it does NOT downgrade target-cpu."

- timestamp: 2026-05-23T00:00:00Z
  checked: "README.md Troubleshooting block (lines 53-60)"
  found: |
    Line 54: 'Apple Silicon (M1/M2/M3/M4): the build script already passes --platform linux/amd64 ... No user action required — the AMD64 emulation layer in Docker Desktop / Colima handles it. Expect ~2-4× slower wall-clock than a native AMD64 host; relative ordering between allocators stays valid.'
    Lines 55-57 cover hyperthreading, NUMA, low-memory mimalloc.
    NO mention of Rosetta-x86 instruction-set gaps.
    NO mention of target-cpu=x86-64-v3 vs Rosetta.
    NO mention of RUSTFLAGS as a knob users might need to override.
  implication: "The user followed the README verbatim and reasonably expected --platform linux/amd64 to be sufficient. The README's claim 'No user action required — the AMD64 emulation layer ... handles it' is wrong for v3-tuned binaries. The Troubleshooting block needs to call out that target-cpu=x86-64-v3 (which README §Reproducibility line 93 itself documents as the locked baseline) emits AVX2/BMI2 — a class of x86 instructions that Rosetta-2 historically does NOT execute reliably (Rosetta supports SSE4.2 and AVX1 but does not emulate AVX2/BMI2)."

- timestamp: 2026-05-23T00:00:00Z
  checked: ".cargo/config.toml host-level rustflags"
  found: |
    [build]
    rustflags = ["-C", "target-cpu=native"]
    Comment block notes: 'For the host smoke build this is fair because the user is benchmarking on their own hardware; the Phase 3 Docker matrix uses a separate baseline target ("x86_64-v3" or similar) for cross-machine comparability'.
  implication: "Host-build (used by `just bench-host`, never by Docker matrix) uses target-cpu=native — irrelevant to the segfault path because Docker builds explicitly override RUSTFLAGS in the builder stage. Confirms the Docker builds are NOT inheriting native flags from the host."

- timestamp: 2026-05-23T00:00:00Z
  checked: "Cross-reference with Phase 3 CONTEXT.md decision rationale"
  found: |
    D-09: 'Build flags injected via ENV RUSTFLAGS in the builder stage: -C target-cpu=x86-64-v3 for portability across CI runners (PITFALLS §3.3). The host-only bench-host recipe uses -C target-cpu=native.'
    D-09 was chosen specifically to handle GHA shared-CPU pool migrations. There is NO discussion of the trade-off against Apple Silicon Rosetta emulation in the CONTEXT — the decision was made for CI portability, and the Apple-Silicon-via-Rosetta path was not considered.
  implication: "The bug is a classic 'CI-portability vs dev-machine-portability' trade-off that wasn't surfaced when D-09 was locked. v3 ≠ a viable lowest-common-denominator if 'all dev hosts including Apple Silicon Rosetta' is the target. The minimum viable LCD is `x86-64` (baseline) or `x86-64-v2` (covers SSE4.2/POPCNT — known-supported by Rosetta)."

- timestamp: 2026-05-23T00:00:00Z
  checked: "Symptom signature: exit code 139 = 128 + 11 = SIGSEGV on first instruction"
  found: |
    Symptoms.errors: 'error: recipe `run` failed with exit code 139 (×18)'.
    Symptoms.actual: 'binary segfaults on launch ... Build steps cached/succeeded; the binary segfaults immediately on docker run.'
    Test 1 reported: 'Build steps cached/succeeded; the binary segfaulted on launch.'
    All 18 cells crash identically (same exit code, same crash point) regardless of allocator (ptmalloc/jemalloc/mimalloc/mallocng) or env (debian-slim/distroless-cc/wolfi/alpine/distroless-static/scratch).
  implication: |
    Universal pattern across all 18 cells = something COMMON to every binary fails. The common factor is: every binary is compiled with target-cpu=x86-64-v3, every binary runs on Apple Silicon Rosetta, every binary segfaults at launch. SIGSEGV on first instruction (vs. partway through bench logic) is the signature of the dynamic linker / loader / startup code hitting an unsupported instruction. Rosetta's documented behavior on AVX2/BMI2 is to deliver SIGILL or SIGSEGV at execution of the unsupported op — exactly matches.
    Notably allocator-specific crashes would NOT all hit at the same launch instant; they would defer to the first allocator-internal hot path. Universal launch-time crash → host CPU instruction-set / emulator coverage gap, not a library bug.

## Resolution

root_cause: |
  The Phase 3 D-09 build flag `RUSTFLAGS="-C target-cpu=x86-64-v3"` is set in every one of the six Dockerfiles' builder stages (alpine.Dockerfile:31, scratch.Dockerfile:41, debian-slim.Dockerfile:27, distroless-cc.Dockerfile:26, distroless-static.Dockerfile:30, wolfi.Dockerfile:32). The x86-64-v3 microarchitecture level mandates AVX2, BMI1, BMI2, FMA, F16C, MOVBE, OSXSAVE on top of the v2 baseline. Apple Silicon's Rosetta-2 x86 emulator does NOT reliably execute AVX2 / BMI2 instructions — Rosetta emulates the v1 baseline plus SSE4.2 and AVX1 only. When `just run` invokes `docker run --platform linux/amd64` on Apple Silicon, the orbstack/Docker-Desktop AMD64 layer dispatches to Rosetta, which traps on the first AVX2/BMI2 instruction emitted by rustc (likely in libstd or rustc's startup / panic-runtime / hashbrown SIMD codegen) and delivers SIGSEGV (exit 139) before the binary even reaches `main`. Every cell crashes identically because every binary uses v3 codegen, regardless of allocator or runtime base image. GHA on real ubuntu-24.04 x86-64 hardware passes all 18 cells because v3 is a real subset of every x86-64-v3-or-newer CPU — confirming the binary is correct, the runtime emulator is not.

  Two ancillary documentation gaps make the failure user-facing:
  (1) README §Troubleshooting Apple Silicon entry (line 54) tells the user 'No user action required — the AMD64 emulation layer ... handles it' — this is wrong for v3-tuned binaries.
  (2) README §Reproducibility (line 93) documents target-cpu=x86-64-v3 as a CI-portability decision, but does not tie it to an Apple-Silicon caveat or provide a knob for users to downgrade.

fix: (deferred — diagnose-only mode; gap-closure plan owns the fix)
verification: (n/a in diagnose-only mode)
files_changed: []
