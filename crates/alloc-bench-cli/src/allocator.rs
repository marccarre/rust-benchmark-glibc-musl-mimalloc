//! Compile-time global allocator selection.
//!
//! Cargo features `alloc-jemalloc` and `alloc-mimalloc` are mutually exclusive.
//! Enabling both is a hard compile error (D-04 in 01-CONTEXT.md).

#[cfg(all(feature = "alloc-jemalloc", feature = "alloc-mimalloc"))]
compile_error!(
    "cargo features `alloc-jemalloc` and `alloc-mimalloc` are mutually exclusive. \
     Build with at most one allocator feature."
);

#[cfg(feature = "alloc-jemalloc")]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

#[cfg(feature = "alloc-mimalloc")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

/// Returns the active allocator's canonical name.
pub const fn name() -> &'static str {
    #[cfg(feature = "alloc-jemalloc")]
    {
        return "jemalloc";
    }
    #[cfg(all(feature = "alloc-mimalloc", not(feature = "alloc-jemalloc")))]
    {
        return "mimalloc";
    }
    #[cfg(not(any(feature = "alloc-jemalloc", feature = "alloc-mimalloc")))]
    {
        if cfg!(target_env = "musl") {
            "mallocng"
        } else if cfg!(target_os = "macos") {
            "libmalloc"
        } else {
            "ptmalloc"
        }
    }
}

/// Defense-in-depth runtime check (D-04). Strictly redundant with the
/// `compile_error!` above; documents the runtime contract.
pub fn assert_mutual_exclusion() {
    if cfg!(all(feature = "alloc-jemalloc", feature = "alloc-mimalloc")) {
        panic!("mutually exclusive allocator features enabled at runtime");
    }
}

/// Emit allocator-internal stats as a `serde_json::Value` for the results JSON.
#[allow(unreachable_code)]
pub fn stats() -> serde_json::Value {
    #[cfg(feature = "alloc-jemalloc")]
    {
        use tikv_jemalloc_ctl::{epoch, stats};
        let _ = epoch::advance();
        return serde_json::json!({
            "kind":      "jemalloc",
            "allocated": stats::allocated::read().unwrap_or(0),
            "resident":  stats::resident::read().unwrap_or(0),
            "retained":  stats::retained::read().unwrap_or(0),
            "active":    stats::active::read().unwrap_or(0),
        });
    }
    #[cfg(feature = "alloc-mimalloc")]
    {
        let mut elapsed_ms: usize = 0;
        let mut user_ms: usize = 0;
        let mut system_ms: usize = 0;
        let mut current_rss: usize = 0;
        let mut peak_rss: usize = 0;
        let mut current_commit: usize = 0;
        let mut peak_commit: usize = 0;
        let mut page_faults: usize = 0;
        unsafe {
            libmimalloc_sys::mi_process_info(
                &mut elapsed_ms,
                &mut user_ms,
                &mut system_ms,
                &mut current_rss,
                &mut peak_rss,
                &mut current_commit,
                &mut peak_commit,
                &mut page_faults,
            );
        }
        return serde_json::json!({
            "kind":           "mimalloc",
            "elapsed_ms":     elapsed_ms,
            "user_ms":        user_ms,
            "system_ms":      system_ms,
            "current_rss":    current_rss,
            "peak_rss":       peak_rss,
            "current_commit": current_commit,
            "peak_commit":    peak_commit,
            "page_faults":    page_faults,
        });
    }
    serde_json::json!({ "kind": "system" })
}
