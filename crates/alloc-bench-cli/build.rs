// Hand-rolled build metadata injection. We don't use vergen because the 1.0.x /
// 9.x crates have an internal version skew (vergen-gitcl re-exports vergen 9
// builders that depend on vergen-lib 9, while vergen-gitcl itself depends on
// vergen-lib 0.1, producing trait-resolution failures). Five `println!`s do the
// same job with no transitive deps.

use std::process::Command;

fn main() {
    let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_string());
    let rustc_version = Command::new(&rustc)
        .arg("--version")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| {
            // Output is like "rustc 1.91.0 (deadbeef 2026-04-14)"; keep only the version.
            s.split_whitespace().nth(1).unwrap_or("unknown").to_string()
        })
        .unwrap_or_else(|| "unknown".to_string());

    let target = std::env::var("TARGET").unwrap_or_else(|_| "unknown".to_string());
    let host = std::env::var("HOST").unwrap_or_else(|_| "unknown".to_string());
    let profile = std::env::var("PROFILE").unwrap_or_else(|_| "unknown".to_string());

    let git_sha = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown".to_string());

    let git_dirty = Command::new("git")
        .args(["status", "--porcelain"])
        .output()
        .ok()
        .map(|o| !o.stdout.is_empty())
        .unwrap_or(false);

    let timestamp = chrono_like_now();

    let rustflags = std::env::var("CARGO_ENCODED_RUSTFLAGS")
        .map(|s| s.replace('\x1f', " "))
        .unwrap_or_default();

    println!("cargo:rustc-env=BUILD_RUSTC_VERSION={}", rustc_version);
    println!("cargo:rustc-env=BUILD_TARGET_TRIPLE={}", target);
    println!("cargo:rustc-env=BUILD_HOST_TRIPLE={}", host);
    println!("cargo:rustc-env=BUILD_PROFILE={}", profile);
    println!("cargo:rustc-env=BUILD_GIT_SHA={}", git_sha);
    println!(
        "cargo:rustc-env=BUILD_GIT_DIRTY={}",
        if git_dirty { "true" } else { "false" }
    );
    println!("cargo:rustc-env=BUILD_TIMESTAMP={}", timestamp);
    println!("cargo:rustc-env=BUILD_RUSTFLAGS={}", rustflags);
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=CARGO_ENCODED_RUSTFLAGS");
}

/// RFC3339 timestamp without pulling chrono into build-deps. Uses
/// `SystemTime::now()` and a tiny formatter — accurate to the second.
fn chrono_like_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // Days from epoch → year/month/day. Quick-and-dirty civil-date math sufficient
    // for build timestamps. Algorithm: Howard Hinnant's chrono_date_algorithms.
    let z = (secs / 86_400) as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let mut y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    if m <= 2 {
        y += 1;
    }
    let secs_in_day = secs % 86_400;
    let hh = secs_in_day / 3_600;
    let mm = (secs_in_day % 3_600) / 60;
    let ss = secs_in_day % 60;
    format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z", y, m, d, hh, mm, ss)
}
