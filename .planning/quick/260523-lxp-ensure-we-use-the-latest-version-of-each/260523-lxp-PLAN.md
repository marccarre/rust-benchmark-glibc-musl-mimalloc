---
quick_task: 260523-lxp
slug: ensure-we-use-the-latest-version-of-each
type: execute
autonomous: true
files_modified:
  - Cargo.toml
  - Cargo.lock
  - rust-toolchain.toml
  - CLAUDE.md
  - docker/alpine.Dockerfile
  - docker/scratch.Dockerfile
  - docker/debian-slim.Dockerfile
  - docker/distroless-cc.Dockerfile
  - docker/distroless-static.Dockerfile
  - docker/wolfi.Dockerfile
  - .github/workflows/bench.yml
  - justfile
must_haves:
  truths:
    - "Every workspace dep in Cargo.toml resolves to its latest semver-compatible version (and latest major where bumps were attempted and accepted)."
    - "rust-toolchain.toml channel matches the latest stable rustc version available on the developer's machine."
    - "All six Dockerfiles' ARG RUST_VERSION default matches rust-toolchain.toml."
    - "Cargo.toml workspace.package.rust-version equals rust-toolchain.toml channel (MSRV-vs-build-pin convention intentionally collapsed; CLAUDE.md updated to document the change)."
    - "All Docker base images (alpine, debian, distroless-cc, distroless-static, wolfi) reference current stable tags / refreshed digests."
    - ".github/workflows/bench.yml pins every action at its latest major version (and dtolnay/rust-toolchain matches the new channel patch)."
    - "cargo fmt --all --check passes."
    - "cargo clippy --workspace --all-targets -- -D warnings passes."
    - "cargo test --workspace passes."
    - "just check-matrix passes (Dockerfile/justfile structural integrity preserved)."
  artifacts:
    - path: "Cargo.toml"
      provides: "Updated workspace.dependencies versions + new rust-version pin"
    - path: "Cargo.lock"
      provides: "Resolved versions after `cargo update` + any major bumps"
    - path: "rust-toolchain.toml"
      provides: "Latest stable rustc channel pin"
    - path: "CLAUDE.md"
      provides: "Updated Conventions section reflecting MSRV equivalent to build-pin"
    - path: ".github/workflows/bench.yml"
      provides: "Updated action major versions + matching dtolnay/rust-toolchain patch"
  key_links:
    - from: "rust-toolchain.toml channel"
      to: "all six docker/*.Dockerfile ARG RUST_VERSION defaults"
      via: "literal version string match"
      pattern: "ARG RUST_VERSION="
    - from: "rust-toolchain.toml channel"
      to: "Cargo.toml workspace.package.rust-version"
      via: "literal version string match (MSRV equivalent to build-pin per locked decision 2)"
      pattern: "rust-version"
    - from: "rust-toolchain.toml channel"
      to: ".github/workflows/bench.yml dtolnay/rust-toolchain@PATCH"
      via: "patch-pin matches channel released patch (e.g. channel=1.91 to @1.91.0)"
      pattern: "dtolnay/rust-toolchain@"
    - from: "Cargo.toml workspace.dependencies"
      to: "Cargo.lock resolved versions"
      via: "cargo update / cargo upgrade resolves deps"
      pattern: "cargo update"
---

<objective>
Refresh every external pin in the repository to its latest stable upstream version: workspace dependencies, Rust toolchain, Docker base images, and GitHub Actions. This is a maintenance sweep — no benchmark logic, profile flags, or scenario code changes.

Purpose: Keep the project current. Reduces accumulated tech debt before the next milestone, ensures CVE patches in transitive deps land, and validates that the v1.0 codebase still cleanly compiles on the latest stable Rust + clippy.

Output: A series of atomic per-axis commits, each independently green under `cargo fmt --check` + `cargo clippy -D warnings` + `cargo test --workspace`, plus a SUMMARY.md documenting deviations (any major bumps reverted, any base-image tag changes).
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@CLAUDE.md
@Cargo.toml
@rust-toolchain.toml
@crates/alloc-bench-cli/Cargo.toml
@crates/alloc-bench-core/Cargo.toml
@crates/alloc-bench-aggregator/Cargo.toml
@docker/alpine.Dockerfile
@docker/scratch.Dockerfile
@docker/debian-slim.Dockerfile
@docker/distroless-cc.Dockerfile
@docker/distroless-static.Dockerfile
@docker/wolfi.Dockerfile
@.github/workflows/bench.yml
@justfile

LATEST KNOWN VERSIONS (heuristic only — executor MUST resolve actual latest at run-time, not hard-code these):

Workspace dependencies (current pin / latest known May 2026):
- clap 4.5 / latest 4.x (resolve via `cargo search clap`)
- serde 1 / latest 1.x
- serde_json 1 / latest 1.x
- hdrhistogram 7.5 / latest 7.x
- libc 0.2 / latest 0.2.x
- rand 0.8 / POTENTIAL MAJOR (rand 0.9 released 2025); attempt major bump
- chrono 0.4 / latest 0.4.x
- anyhow 1 / latest 1.x
- tikv-jemallocator 0.6 / latest 0.6.x (0.6.1 noted in CLAUDE.md)
- tikv-jemalloc-ctl 0.6 / latest 0.6.x
- mimalloc 0.1 / latest 0.1.x (0.1.43 noted in CLAUDE.md)
- libmimalloc-sys 0.1 / latest 0.1.x
- num_cpus 1.16 / latest 1.x
- crossbeam-channel 0.5 / latest 0.5.x
- axum 0.8 / POTENTIAL MAJOR (check for 0.9+); attempt major bump if available
- tokio 1 / latest 1.x
- tower 0.5 / POTENTIAL MAJOR (check for 0.6+); attempt major bump if available
- reqwest 0.12 / latest 0.12.x
- rayon 1 / latest 1.x
- tinytemplate 1 / latest 1.x
- glob 0.3 / latest 0.3.x

NOTE: vergen is in CLAUDE.md TL;DR but NOT currently in Cargo.toml. Do NOT add it — this is a refresh, not an addition.

Member dev-dependencies (also need refresh):
- assert_cmd 2 (in alloc-bench-cli + alloc-bench-aggregator)
- tempfile 3 (in alloc-bench-cli + alloc-bench-aggregator)
- predicates 3 (in alloc-bench-aggregator)

Rust toolchain: rust-toolchain.toml channel = "1.91" / latest stable. As of 2026-05, 1.91.x is current; if a newer minor like 1.92 has shipped, use it. Resolve via `rustup update stable && rustc +stable --version --verbose`.

GHA actions (current pin / check for newer major):
- actions/checkout@v6 / check for v7+
- actions/upload-artifact@v7 / check for v8+
- actions/download-artifact@v8 / check for v9+
- docker/build-push-action@v7 / check for v8+
- docker/setup-buildx-action@v4 / check for v5+
- Swatinem/rust-cache@v2 / check for v3+
- extractions/setup-just@v4 / check for v5+
- dtolnay/rust-toolchain@1.91.0 / patch-pin matches the chosen rust-toolchain.toml channel's latest patch

Docker base images:
- alpine:3.20 / latest 3.x (resolve via `docker pull alpine:3 && docker image inspect alpine:3 --format '{{index .RepoTags 0}}'`)
- debian:bookworm-slim / latest patch (forever-current tag, refreshes implicitly on docker pull — NO source edit)
- gcr.io/distroless/cc-debian12:nonroot / forever-current tag (NO source edit)
- gcr.io/distroless/static-debian12:nonroot / forever-current tag (NO source edit)
- cgr.dev/chainguard/wolfi-base@sha256:0cff4df29a6597173dc8b813787318150141eb96ac783dc3ff4f5ff52c49a1e2 / refresh digest via `docker buildx imagetools inspect cgr.dev/chainguard/wolfi-base:latest | grep '^Digest:'`

CLAUDE.MD CONVENTIONS BULLET TO REPLACE (per locked decision 2):

Existing bullet (find it under "## Conventions" in CLAUDE.md):
```
- **rustc pin source-of-truth:** `rust-toolchain.toml` (`channel = "1.91"`); the workspace `Cargo.toml` `rust-version = "1.83"` is the **MSRV** (minimum supported version for downstream consumers), NOT the build-time pin. The two fields have distinct semantics — do not conflate them.
```

Replace its full content with (substituting NEW_VERSION with the actual chosen channel literal, e.g. `1.91` or `1.92`):
```
- **rustc pin source-of-truth:** `rust-toolchain.toml` (`channel = "NEW_VERSION"`) is the build pin; `Cargo.toml` `rust-version = "NEW_VERSION"` is set to match (MSRV ≡ build-pin). This is an intentional break of the prior MSRV-as-downstream-floor convention — the project is a benchmark suite, not a library, so there is no downstream MSRV contract to honor. Refreshed 2026-05-23 (quick task 260523-lxp).
```
</context>

<tasks>

<task type="auto">
  <name>Task 1: Refresh workspace dependencies (cargo update + best-effort major bumps)</name>
  <files>Cargo.toml, Cargo.lock, crates/alloc-bench-cli/Cargo.toml, crates/alloc-bench-aggregator/Cargo.toml</files>
  <action>
Refresh `Cargo.toml` workspace.dependencies and member dev-dependencies to latest versions. Per locked decision 1, attempt major bumps where stable upstream releases exist.

Step 1 — Install cargo-edit if missing. Run `command -v cargo-upgrade` and if absent, run `cargo install cargo-edit --locked` to make `cargo upgrade` available.

Step 2 — Establish baseline. Run `cargo build --workspace --release` to confirm clean tree before any change. If this fails, STOP and report.

Step 3 — Patch / minor bumps via lockfile. Run `cargo update --workspace`. This refreshes Cargo.lock without touching Cargo.toml. Verify with `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace`. Commit as a separate atomic commit: `chore(lxp): cargo update --workspace (lockfile refresh)`.

Step 4 — Manifest version bumps (semver-compatible). For each workspace.dependency in `Cargo.toml`, run `cargo upgrade --dry-run` to discover compatible upgrades, then re-run without `--dry-run` to apply. This bumps the version literals in Cargo.toml itself (e.g. `clap = "4.5"` to `clap = "4.6"` if a newer minor exists). Re-verify with all three gates. Commit as `chore(lxp): bump workspace.dependencies to latest semver-compatible`.

Step 5 — Major bumps (best-effort, per locked decision 1). Identified candidates from research: `rand` 0.8 to 0.9, `axum` 0.8 to 0.9 (if released), `tower` 0.5 to 0.6 (if released). Other crates may also have new majors — discover via `cargo upgrade --incompatible --dry-run`. For EACH major-bump candidate:
  a. Run `cargo upgrade --incompatible --package <crate>` to attempt the major bump (one crate at a time so failures are bisectable).
  b. Run `cargo build --workspace --release` to surface compile errors.
  c. If compile succeeds, run `cargo clippy --workspace --all-targets -- -D warnings` and `cargo test --workspace`.
  d. If ALL gates pass, the major bump is accepted — note in SUMMARY.md "Accepted Major Bumps".
  e. If ANY gate fails, IMMEDIATELY revert that single crate's bump (edit Cargo.toml back to the prior major literal, then `cargo update --package <crate>`). DO NOT attempt to fix the calling code in this sweep — the locked decision 1 rollback policy is "revert any crate whose major bump fails". Document in SUMMARY.md "Rejected Major Bumps" with the failure mode (compile error / clippy lint / test failure).
  f. Commit each accepted major bump as its own atomic commit: `chore(lxp): bump <crate> <old-major> to <new-major>`. Multiple successful bumps may be grouped into one commit if they were exercised together.

Step 6 — Member-crate dev-dependencies. Inspect `crates/alloc-bench-cli/Cargo.toml` (assert_cmd, tempfile) and `crates/alloc-bench-aggregator/Cargo.toml` (assert_cmd, predicates, tempfile). Run `cargo upgrade --dry-run --manifest-path crates/alloc-bench-cli/Cargo.toml` and similarly for aggregator. Apply if upgrades exist. Re-verify gates. Commit if any change: `chore(lxp): bump dev-dependencies to latest`.

Constraints:
- DO NOT touch `[profile.release]` flags (lto, codegen-units, opt-level, strip, debug, panic, overflow-checks). Locked by Phase-2 review CR-01.
- DO NOT touch the `glob = "0.3"` slopcheck false-positive comment (Cargo.toml:33).
- DO NOT touch the `panic = "unwind"` comment block (Cargo.toml:42-51).
- DO NOT add new dependencies (e.g. `vergen`) even if listed in CLAUDE.md TL;DR.
- Preserve all existing feature flags (`features = ["derive"]`, `default-features = false`, etc.) verbatim unless the new major version REQUIRES a feature flag rename. If a feature is renamed, document in SUMMARY.md.
- Cargo.toml's `[workspace.package]` `rust-version = "1.83"` is intentionally OUT OF SCOPE for this task — Task 2 owns it.

Verification gates after EACH commit (run all three; do not proceed if any fail):
- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
  </action>
  <verify>
    <automated>cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace</automated>
  </verify>
  <done>
- `cargo update --workspace` has run and Cargo.lock is updated.
- Every workspace.dependency in Cargo.toml shows its latest semver-compatible version literal (verified by `cargo upgrade --dry-run` reporting no remaining compatible upgrades).
- Every attempted major bump is either accepted (build+clippy+test all pass) or reverted (with rejection rationale staged for SUMMARY.md).
- Member dev-dependencies refreshed.
- Three verification gates pass on the final state.
- Atomic commits land in git history with `chore(lxp):` prefix per CLAUDE.md Conventions.
  </done>
</task>

<task type="auto">
  <name>Task 2: Bump Rust toolchain channel + collapse MSRV to match build-pin</name>
  <files>rust-toolchain.toml, Cargo.toml, CLAUDE.md, docker/alpine.Dockerfile, docker/scratch.Dockerfile, docker/debian-slim.Dockerfile, docker/distroless-cc.Dockerfile, docker/distroless-static.Dockerfile, docker/wolfi.Dockerfile, .github/workflows/bench.yml, justfile</files>
  <action>
Bump rustc to latest stable across ALL pin sites simultaneously. This is the highest-blast-radius change in the sweep — every pin must move together or builds will diverge.

Step 1 — Resolve the new version literal. Run `rustup update stable` on the developer's host, then `rustc +stable --version --verbose | head -1` to capture the literal (e.g. `rustc 1.92.0 (abc123 2026-04-15)` resolves to channel literal `1.92` and patch literal `1.92.0` for the dtolnay action). Record both:
  - CHANNEL — the rust-toolchain.toml form (e.g. `1.92`)
  - PATCH — the dtolnay/rust-toolchain action form (e.g. `1.92.0`)

If `rustc +stable --version` reports the SAME version that's already pinned (i.e. `1.91`), the toolchain bump is a no-op — but Task 2 STILL must execute the MSRV-collapse half (Step 6) because that's a separate locked decision.

Step 2 — Update `rust-toolchain.toml`. Replace `channel = "1.91"` with `channel = "CHANNEL"`. Keep `components = ["rustfmt", "clippy"]` unchanged.

Step 3 — Update all six Dockerfiles' `ARG RUST_VERSION` default. Files and approximate lines:
  - docker/alpine.Dockerfile (line 5)
  - docker/scratch.Dockerfile (line 17)
  - docker/debian-slim.Dockerfile (line 9)
  - docker/distroless-cc.Dockerfile (line 9)
  - docker/distroless-static.Dockerfile (line 6)
  - docker/wolfi.Dockerfile (line 16)
For each, replace `ARG RUST_VERSION=1.91` with `ARG RUST_VERSION=CHANNEL`. Also update surrounding comments that mention "rust-toolchain.toml=1.91" or "channel = \"1.91\"" — these are documentation pointers, keep them in sync with the new literal.

Step 4 — Update `.github/workflows/bench.yml`:
  - Line 88: `dtolnay/rust-toolchain@1.91.0` to `dtolnay/rust-toolchain@PATCH` (patch-pinned per CLAUDE.md Conventions GHA action pinning)
  - Line 175: `RUST_VERSION=1.91` to `RUST_VERSION=CHANNEL` (the build-arg)
  - Line 224: `dtolnay/rust-toolchain@1.91.0` to `dtolnay/rust-toolchain@PATCH` (second occurrence in aggregate job)
  - Comment block lines 41-42 reference "rustc pin is 1.91 (NOT 1.83)" — update both literals to reflect the new channel.

Step 5 — Update `justfile`. Line 106 references `--build-arg RUST_VERSION=1.91`. Replace with `--build-arg RUST_VERSION=CHANNEL`.

Step 6 — Collapse MSRV (per locked decision 2). Edit `Cargo.toml`:
  - Line 7: `rust-version = "1.83"` to `rust-version = "CHANNEL"`. This intentionally collapses the MSRV-vs-build-pin distinction.

Step 7 — Update CLAUDE.md (per locked decision 2). Find the Conventions-section bullet starting `**rustc pin source-of-truth:**` and replace its full text with the new wording shown in the plan's `<context>` block, substituting NEW_VERSION with the actual CHANNEL literal.

Step 8 — Verify (Docker layer is skipped per locked decision 6, so use grep + structural checks):
  - Run `cat docker/*.Dockerfile | grep '^ARG RUST_VERSION='` and confirm every line shows the same CHANNEL literal (six lines expected).
  - Run `grep -nE 'RUST_VERSION|rust-toolchain' .github/workflows/bench.yml` and confirm every occurrence is updated (3 sites: build-arg + 2 dtolnay refs).
  - Run `grep -n 'RUST_VERSION' justfile` and confirm the build recipe is updated.
  - Run `grep -n '^rust-version' Cargo.toml` and confirm the new literal.
  - Run `cat rust-toolchain.toml` and confirm the new literal.
  - Run `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace` — these now exercise the new toolchain. If clippy fires NEW lints from a newer rustc, address them via `cargo fix --workspace --allow-dirty --allow-staged` for mechanical fixes; document any `#[allow(...)]` exceptions in SUMMARY.md with justification.
  - Run `just check-matrix` to confirm the structural integrity of the matrix-cells parser is unaffected.

Step 9 — Commit. Single atomic commit because every pin site MUST move together (a partial commit leaves the repo with mismatched build-pin vs MSRV vs Docker arg, violating the rust-toolchain.toml invariant in CLAUDE.md):
  `chore(lxp): bump rustc to CHANNEL across rust-toolchain.toml, Cargo.toml MSRV, all Dockerfiles, GHA, justfile`

Constraints:
- DO NOT touch any code in `crates/` for this task. New rustc lints — if any — should be addressed by minimal mechanical fixes (e.g. `cargo fix --workspace --allow-dirty --allow-staged`) inside this same commit. Logic changes are out of scope.
- DO NOT alter `.github/workflows/bench.yml` matrix structure (`include:` block, scenario list, cache keys) — only the version literal in the build-arg.
- DO NOT modify the Phase 5 D-19 timeout literals (`timeout-minutes: 15/30/15`) or the matrix-cells justfile block.
- The MSRV collapse intentionally breaks the existing CLAUDE.md convention. The CLAUDE.md update IS that documentation — the user has explicitly approved.
- If the new rustc introduces a `clippy::correctness`-tier lint that requires nontrivial code changes, treat that as a hard rollback signal — revert the toolchain bump and report.
  </action>
  <verify>
    <automated>cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && just check-matrix && test "$(grep -h '^ARG RUST_VERSION=' docker/*.Dockerfile | sort -u | wc -l)" -eq 1</automated>
  </verify>
  <done>
- `rust-toolchain.toml` channel literal updated to latest stable (or no-op if already current).
- All six Dockerfiles' `ARG RUST_VERSION` defaults match `rust-toolchain.toml`.
- `.github/workflows/bench.yml` updates: dtolnay/rust-toolchain patch (×2 occurrences), build-arg RUST_VERSION (×1), comment block.
- `justfile` build recipe updated.
- `Cargo.toml` `rust-version` collapsed to match build-pin.
- `CLAUDE.md` Conventions bullet updated to reflect MSRV equivalent to build-pin policy with date.
- `cargo fmt --check` + `cargo clippy -D warnings` + `cargo test --workspace` + `just check-matrix` all pass on the new toolchain.
- Single atomic commit lands the bump.
  </done>
</task>

<task type="auto">
  <name>Task 3: Refresh Docker base image tags + wolfi digest</name>
  <files>docker/alpine.Dockerfile, docker/wolfi.Dockerfile</files>
  <action>
Refresh non-Rust Docker base images per locked decision 4. The debian-slim and distroless-cc / distroless-static Dockerfiles already use forever-current tags (`bookworm-slim`, `:nonroot`) so a `docker pull` at build time will get the latest patches without any source change — those Dockerfiles need NO edits in this task. Only `alpine:3.20` (pinned to a numbered minor) and the `wolfi-base@sha256:0cff...` (pinned to a frozen digest) need source-level edits.

Step 1 — Resolve the latest stable Alpine 3.x minor. Run `docker pull alpine:3` to fetch the floating tag, then `docker image inspect alpine:3 --format '{{index .RepoTags 0}}'`. Cross-reference with https://alpinelinux.org/releases/ — the latest stable 3.x as of 2026-05 is likely 3.21 or 3.22. Record the literal as ALPINE_VERSION (e.g. `3.21`).

If the resolved version equals `3.20` (no upstream bump since project init), this step is a no-op — proceed to wolfi.

Step 2 — Update `docker/alpine.Dockerfile`:
  - Line 66: `FROM alpine:3.20 AS runtime` to `FROM alpine:ALPINE_VERSION AS runtime`
  - Line 78 ENV: `ENV DOCKER_IMAGE=alpine:3.20` to `ENV DOCKER_IMAGE=alpine:ALPINE_VERSION`
  - Update the line 65 comment `# ─── Stage 4: runtime — alpine:3.20 (matches success criterion 2 literal) ──` to reflect the new literal.

Step 3 — Resolve the latest wolfi-base digest. Per the comment in `docker/wolfi.Dockerfile:8-9`, the canonical command is:
  `docker buildx imagetools inspect cgr.dev/chainguard/wolfi-base:latest | grep '^Digest:'`
Run this command and capture the SHA256 digest as WOLFI_DIGEST (the full `sha256:<hex>` string). The instruction in the file specifies pinning the **manifest-list digest**, NOT a per-arch manifest — the floating-tag inspect output gives the manifest-list by default, so use it directly.

If the resolved digest equals the existing `0cff4df29a6597173dc8b813787318150141eb96ac783dc3ff4f5ff52c49a1e2` (no upstream rebuild), this is a no-op.

Step 4 — Update `docker/wolfi.Dockerfile`:
  - Line 9 (comment in the digest-block): replace the literal `sha256:0cff4df29a6597173dc8b813787318150141eb96ac783dc3ff4f5ff52c49a1e2` with the new digest string.
  - Line 8 (date in the comment): update `2026-05-19` to today's date `2026-05-23` to truthfully reflect when the digest was captured.
  - Line 68 `FROM cgr.dev/chainguard/wolfi-base@sha256:0cff... AS runtime` to `FROM cgr.dev/chainguard/wolfi-base@WOLFI_DIGEST AS runtime`
  - Line 80 `ENV DOCKER_IMAGE=cgr.dev/chainguard/wolfi-base@sha256:0cff...` to `ENV DOCKER_IMAGE=cgr.dev/chainguard/wolfi-base@WOLFI_DIGEST`

Step 5 — Verify (no docker buildx exercise per locked decision 6, so use grep + structural checks):
  - Run `grep -n 'alpine:' docker/alpine.Dockerfile` — every occurrence should show ALPINE_VERSION, none should show the old `3.20`.
  - Run `grep -n 'wolfi-base@' docker/wolfi.Dockerfile` — every occurrence (including the comment, the FROM, and the ENV) should show the new digest, none should show `0cff4df...`.
  - Run `just check-matrix` to confirm the matrix-cells parser is still happy.
  - Run `grep -c '^FROM' docker/alpine.Dockerfile docker/wolfi.Dockerfile` — alpine should show 4 (chef/planner/builder/runtime), wolfi should show 4.
  - Run `cargo fmt --all --check` + `cargo clippy --workspace --all-targets -- -D warnings` + `cargo test --workspace` — these are inert with respect to the Dockerfile changes but must remain green.

Step 6 — Commit single atomic. Group both base-image changes into one commit since they're the same axis:
  `chore(lxp): refresh alpine 3.20 to ALPINE_VERSION + wolfi-base digest <new_short_sha>`

Constraints:
- DO NOT modify `docker/debian-slim.Dockerfile`, `docker/distroless-cc.Dockerfile`, or `docker/distroless-static.Dockerfile` — their tags are already forever-current (`bookworm-slim`, `:nonroot`) and refresh implicitly on the next `docker pull`.
- DO NOT alter the wolfi.Dockerfile comment block lines 4-13 EXCEPT the digest literal and date — the rationale text (manifest-list vs per-arch, RESEARCH §Pitfall 6 reference) is locked by Phase 3 D-04 and remains accurate.
- DO NOT change the `nonroot` tag suffix on distroless images — the project relies on UID 65532 per Pitfall §4.
- DO NOT modify the `.dive-ci` thresholds even if a base-image bump nudges image size — that's a Phase 5 ORCH-05 invariant; if dive fails on the new base, that's an investigation, not a threshold relax.
  </action>
  <verify>
    <automated>just check-matrix && test "$(grep -c '^FROM alpine:' docker/alpine.Dockerfile)" -eq 1 && test "$(grep -c '^FROM cgr.dev/chainguard/wolfi-base@sha256:' docker/wolfi.Dockerfile)" -eq 1 && cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings</automated>
  </verify>
  <done>
- `docker/alpine.Dockerfile` references the latest stable Alpine 3.x minor (or no-op if already current).
- `docker/wolfi.Dockerfile` references a refreshed wolfi-base manifest-list digest captured 2026-05-23 (or no-op if already current).
- All four other Dockerfiles unchanged (their floating tags refresh implicitly).
- `just check-matrix` passes.
- Single atomic commit recording the base-image refresh.
  </done>
</task>

<task type="auto">
  <name>Task 4: Bump GitHub Actions to latest major versions</name>
  <files>.github/workflows/bench.yml</files>
  <action>
Refresh every action pin in `.github/workflows/bench.yml` to its latest major version per locked decision 5. The `dtolnay/rust-toolchain@PATCH` action stays patch-pinned and was already updated in Task 2 — this task does NOT re-touch it.

Step 1 — Resolve latest majors for each action. The actions in scope:
  - `actions/checkout@v6` (lines 86, 138, 222) — check for v7+
  - `actions/upload-artifact@v7` (lines 200, 256) — check for v8+
  - `actions/download-artifact@v8` (line 235) — check for v9+
  - `docker/setup-buildx-action@v4` (line 140) — check for v5+
  - `docker/build-push-action@v7` (line 164) — check for v8+
  - `Swatinem/rust-cache@v2` (lines 92, 147, 226) — check for v3+
  - `extractions/setup-just@v4` (lines 96, 142, 230) — check for v5+

Resolution method: For each action, run `gh api repos/<owner>/<repo>/releases/latest --jq '.tag_name'` (e.g. `gh api repos/actions/checkout/releases/latest --jq '.tag_name'`). The tag will be like `v7.0.0` — strip the patch portion to get the major (`v7`).

If `gh` CLI is unavailable, fall back to `curl -sSL https://api.github.com/repos/<owner>/<repo>/releases/latest | jq -r '.tag_name'` (no auth needed for public repos within the rate limit).

Step 2 — For each action with an available major bump, edit `.github/workflows/bench.yml` and update ALL occurrences. Use `grep -n '<action>@v<old>' .github/workflows/bench.yml` first to enumerate every occurrence, then edit each. The file has multiple jobs (`pre-bench`, `bench-matrix`, `aggregate`) and the same action appears in 2-3 places.

Per CLAUDE.md Conventions ("GHA action pinning"): always pin to the major (e.g. `@v7`), NEVER to a specific minor or patch (NEVER `@v7.0.0` or `@v7.1`), NEVER to `@latest` or `@main`. The sole exception is `dtolnay/rust-toolchain` which is patch-pinned per project convention — Task 2 owns that one.

Step 3 — Validate workflow syntax. The repo does not include `actionlint` as a dev tool by default; fall back to a structural grep audit:
  - Run `grep -nE '@v[0-9]+\b' .github/workflows/bench.yml` — every action reference must match `@v<digits>` only, no `@latest`, no `@main`, no `@v<digits>.<digits>` (the latter would indicate accidental over-pinning).
  - Run `grep -c 'uses: actions/checkout@' .github/workflows/bench.yml` and confirm 3 (one per job).
  - Run `grep -c 'uses: Swatinem/rust-cache@' .github/workflows/bench.yml` and confirm 3.
  - Run `grep -c 'uses: extractions/setup-just@' .github/workflows/bench.yml` and confirm 3.
  - Run `grep -c 'uses: dtolnay/rust-toolchain@' .github/workflows/bench.yml` and confirm 2.
  - Run `grep -c 'uses: docker/' .github/workflows/bench.yml` and confirm 2 (setup-buildx-action + build-push-action).
  - If `actionlint` IS installed (`command -v actionlint` returns 0), run `actionlint .github/workflows/bench.yml` and address any errors.

Step 4 — Sanity-check breaking changes between major versions. For each action that bumped a major, briefly check the release notes (the same `gh api` call's body field). Likely candidates of concern:
  - `docker/build-push-action`: between v6 and v7 the `cache-from`/`cache-to` syntax was unchanged; if v8 lands and changes that syntax, fix in-place.
  - `actions/upload-artifact`: between v3 to v4 the unique-name-per-job model became mandatory; if v8 lands and changes the artifact API again, fix in-place.
If a breaking change requires nontrivial rewrites in this workflow file, that crate-level major bump is REJECTED — revert that single action to its prior major and document in SUMMARY.md "Rejected GHA Action Bumps".

Step 5 — Verify project-level gates remain green:
  - `cargo fmt --all --check`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace`
  - `just check-matrix`

Step 6 — Commit single atomic per locked decision 1 (one commit for GHA actions):
  `ci(lxp): bump GHA actions to latest majors`

Constraints:
- DO NOT touch `dtolnay/rust-toolchain@PATCH` — Task 2 owns it.
- DO NOT alter the matrix `include:` block — its 18 cells are structurally locked per CLAUDE.md "Cross-libc rejection".
- DO NOT change `permissions:`, `concurrency:`, `env:`, or `timeout-minutes:` literals — these are Phase-5 D-19 / WR-01 / Open-Questions invariants.
- DO NOT alter the `cache-from`/`cache-to` scope expression — Phase 5 D-07 / RESEARCH §Pattern 3 invariant.
- If a major bump on `actions/upload-artifact` or `actions/download-artifact` changes artifact name resolution semantics (e.g. v8 deprecates `merge-multiple`), STOP and revert that bump. The aggregate-job glob+merge pattern is locked by Phase 5 RESEARCH §Pattern 2.
  </action>
  <verify>
    <automated>cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && just check-matrix && ! grep -E '@(latest|main)\b' .github/workflows/bench.yml</automated>
  </verify>
  <done>
- Every GHA action in `.github/workflows/bench.yml` is pinned to its latest major (or no-op if already current), preserving the `@v<major>` pattern.
- Any rejected major bump is documented in SUMMARY.md with the breaking-change rationale.
- `dtolnay/rust-toolchain@PATCH` remains patch-pinned and matches Task 2's chosen rustc patch.
- Workflow structural integrity preserved (matrix include block, permissions, concurrency, timeouts, cache scopes all unchanged).
- `cargo fmt --check` + `cargo clippy -D warnings` + `cargo test --workspace` + `just check-matrix` all pass.
- Single atomic commit `ci(lxp): bump GHA actions to latest majors`.
  </done>
</task>

</tasks>

<verification>
Final whole-repository verification (run after all four tasks land):

1. `cargo fmt --all --check` — formatting clean.
2. `cargo clippy --workspace --all-targets -- -D warnings` — no clippy warnings on the new toolchain.
3. `cargo test --workspace` — all unit + integration tests pass.
4. `just check-matrix` — Dockerfile/justfile structural integrity preserved.
5. `cat docker/*.Dockerfile | grep '^ARG RUST_VERSION=' | sort -u | wc -l` — must equal 1 (all six files agree).
6. `grep -c '^rust-version' Cargo.toml` and `grep -c '^channel' rust-toolchain.toml` — both must equal 1, and the literals must match (MSRV equivalent to build-pin per locked decision 2).
7. `grep -E '@(latest|main)\b' .github/workflows/bench.yml` — must produce no output (no floating-tag pins).
8. `git log --oneline | head -10` — atomic commits with `chore(lxp):` and `ci(lxp):` prefixes per the per-axis decomposition in locked decision 1.

A SUMMARY.md MUST be written to the same quick-task directory documenting:
- Final resolved versions for each axis (rustc, alpine, wolfi digest, GHA action majors).
- Accepted major bumps (crate name, old major, new major, why it worked).
- Rejected major bumps (crate name, attempted target, failure mode, current pin retained).
- Any clippy fixes applied alongside the toolchain bump.
- Any noteworthy CLAUDE.md changes (the MSRV convention bullet rewrite).
</verification>

<success_criteria>
- All four tasks committed atomically with the right conventional-commit prefixes.
- Verification gate (`cargo fmt --check && cargo clippy -D warnings && cargo test --workspace && just check-matrix`) passes on `HEAD` after the final commit.
- No floating-tag GHA pins (`@latest`, `@main`) anywhere in `.github/workflows/`.
- All six Dockerfile `ARG RUST_VERSION` defaults agree.
- `Cargo.toml` `rust-version` equals `rust-toolchain.toml` channel.
- CLAUDE.md Conventions section reflects the MSRV equivalent to build-pin policy with date 2026-05-23.
- SUMMARY.md written to `.planning/quick/260523-lxp-ensure-we-use-the-latest-version-of-each/260523-lxp-SUMMARY.md` documenting accepted/rejected bumps.
- Docker layer NOT exercised (per locked decision 6) — verified by grep + just check-matrix only.
- Benchmark logic, profile.release flags, and scenario code untouched.
</success_criteria>

<output>
Create `.planning/quick/260523-lxp-ensure-we-use-the-latest-version-of-each/260523-lxp-SUMMARY.md` when done.

The SUMMARY.md MUST follow the standard quick-task summary template and include the sections:
- Resolved Versions (table: axis | before | after | source-of-truth file)
- Accepted Major Bumps (list with rationale)
- Rejected Major Bumps (list with failure mode)
- Deviations from Plan (anything that didn't go to plan)
- Verification Evidence (paste the final `cargo fmt --check && cargo clippy && cargo test && just check-matrix` output)
- Commits Landed (table: SHA | subject | axis)
</output>
