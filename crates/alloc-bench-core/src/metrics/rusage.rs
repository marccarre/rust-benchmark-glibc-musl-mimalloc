use crate::output::Rusage;

pub fn read_rusage() -> anyhow::Result<Rusage> {
    let mut ru: libc::rusage = unsafe { std::mem::zeroed() };
    let ret = unsafe { libc::getrusage(libc::RUSAGE_SELF, &mut ru) };
    anyhow::ensure!(ret == 0, "getrusage failed");

    let peak_rss_kb = {
        #[cfg(target_os = "macos")]
        {
            ru.ru_maxrss as u64 / 1024
        }
        #[cfg(not(target_os = "macos"))]
        {
            ru.ru_maxrss as u64
        }
    };

    let timeval_to_secs = |tv: libc::timeval| tv.tv_sec as f64 + tv.tv_usec as f64 / 1_000_000.0;

    Ok(Rusage {
        user_time_s: timeval_to_secs(ru.ru_utime),
        sys_time_s: timeval_to_secs(ru.ru_stime),
        minor_faults: ru.ru_minflt as u64,
        major_faults: ru.ru_majflt as u64,
        voluntary_csw: ru.ru_nvcsw as u64,
        involuntary_csw: ru.ru_nivcsw as u64,
        peak_rss_kb,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rusage_returns_positive_rss() {
        let r = read_rusage().expect("getrusage failed");
        assert!(r.peak_rss_kb > 0, "peak_rss_kb should be > 0");
    }
}
