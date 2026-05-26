//! Phase 6 GUARD-01: pin SHA-256 of `crates/alloc-bench-core/src/output.rs`
//! to its v1.0 freeze. Guards the v1 schema contract (Phase 1 D-11 +
//! CLAUDE.md Conventions: "Aggregator decorate-not-rewrite").
//!
//! When this test fails, the contributor must either:
//!   (a) prove the diff to output.rs is sidecar-only / additive-with-skip-
//!       serializing-if (and the existing `run_canonical_shape_snapshot`
//!       in output.rs's in-file `mod tests` still passes byte-equivalence),
//!       AND bump the pinned hash with a one-line commit message
//!       explaining the additive change; OR
//!   (b) explicitly migrate the schema to v2 (bump SCHEMA_VERSION to 2 in
//!       output.rs:3 in the SAME commit, regenerate goldens, and bump the
//!       pinned hash).
//!
//! There is no third option; "I just refactored, please trust me" is not.

use sha2::{Digest, Sha256};

// Lowercase hex; matches both 'sha256sum' coreutils and 'shasum -a 256' macOS output.
// Hash assumes LF line endings; verified on macOS/Linux via 'sha256sum'.
// To recompute (after an intentional, justified change):
//   sha256sum crates/alloc-bench-core/src/output.rs
#[rustfmt::skip]
const V1_OUTPUT_RS_SHA256: &str = "1bcfb91252eddc2710222abd46b031b85d91267d97a0874fa78d042c15f99a84";

mod tests {
    use super::*;

    #[test]
    fn v1_schema_output_rs_is_frozen() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/output.rs");
        let bytes = std::fs::read(&path)
            .unwrap_or_else(|e| panic!("reading {} for SHA-256 freeze test: {e}", path.display()));
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        let actual = format!("{:x}", hasher.finalize());

        assert_eq!(
            actual,
            V1_OUTPUT_RS_SHA256,
            "\n\nv1 schema in {} has changed.\n\
             If this is intentional:\n  \
               (a) sidecar-only / additive-Option-with-skip-serializing-if change \
                   that preserves byte-equivalence of all v1.0 fixtures: bump the \
                   pinned hash V1_OUTPUT_RS_SHA256 in this file and document in the \
                   commit message; OR\n  \
               (b) v1 -> v2 schema migration: bump SCHEMA_VERSION in output.rs:3 \
                   AND the pinned hash AND regenerate all golden fixtures in the \
                   SAME commit.\n\
             Computed: {}\n\
             Expected: {}\n",
            path.display(),
            actual,
            V1_OUTPUT_RS_SHA256,
        );
    }
}
