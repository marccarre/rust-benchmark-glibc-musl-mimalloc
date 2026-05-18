pub const RUSTC_VERSION: &str = env!("BUILD_RUSTC_VERSION");
pub const HOST_TRIPLE: &str = env!("BUILD_HOST_TRIPLE");
pub const TARGET_TRIPLE: &str = env!("BUILD_TARGET_TRIPLE");
pub const PROFILE: &str = env!("BUILD_PROFILE");
pub const GIT_SHA: &str = env!("BUILD_GIT_SHA");
pub const GIT_DIRTY: &str = env!("BUILD_GIT_DIRTY");
pub const BUILD_TIMESTAMP: &str = env!("BUILD_TIMESTAMP");
pub const RUSTFLAGS: &str = env!("BUILD_RUSTFLAGS");
pub const CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// IN-01 (Phase-2 review): truncate `GIT_SHA` to 8 chars, gracefully
/// handling shorter SHAs (e.g., empty string from a shallow checkout).
/// Hoisted out of `main.rs` and `run.rs` so the truncation policy lives
/// in one place — change the `.min(8)` here once and both call sites
/// follow.
pub fn short_sha() -> &'static str {
    let sha = GIT_SHA;
    &sha[..sha.len().min(8)]
}
