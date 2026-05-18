/// Read current RSS from /proc/self/statm in kB.
/// Returns 0 on non-Linux platforms — use getrusage for peak RSS instead.
pub fn read_rss_kb() -> anyhow::Result<u64> {
    #[cfg(target_os = "linux")]
    {
        let content = std::fs::read_to_string("/proc/self/statm")?;
        let resident_pages: u64 = content
            .split_whitespace()
            .nth(1)
            .ok_or_else(|| anyhow::anyhow!("/proc/self/statm parse error"))?
            .parse()?;
        let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) } as u64;
        Ok(resident_pages * page_size / 1024)
    }
    #[cfg(not(target_os = "linux"))]
    {
        Ok(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rss_kb_platform_expectation() {
        let rss = read_rss_kb().expect("read_rss_kb failed");
        if cfg!(target_os = "linux") {
            assert!(rss > 0, "expected rss > 0 on Linux");
        } else {
            assert_eq!(rss, 0, "expected 0 on non-Linux");
        }
    }
}
