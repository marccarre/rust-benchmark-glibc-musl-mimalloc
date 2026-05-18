use crate::output::Env;

pub fn read_env() -> anyhow::Result<Env> {
    Ok(Env {
        os: std::env::consts::OS.to_string(),
        os_version: read_os_version(),
        docker_image: std::env::var("DOCKER_IMAGE").ok(),
        cpu_model: read_cpu_model(),
        cpu_count: num_cpus::get() as u32,
        memory_total_kb: read_memory_total_kb(),
    })
}

fn read_os_version() -> String {
    #[cfg(target_os = "linux")]
    if let Ok(content) = std::fs::read_to_string("/proc/version") {
        if let Some(line) = content.lines().next() {
            return line.to_string();
        }
    }

    #[cfg(target_os = "macos")]
    if let Ok(out) = std::process::Command::new("uname").arg("-r").output() {
        if out.status.success() {
            return String::from_utf8_lossy(&out.stdout).trim().to_string();
        }
    }

    "unknown".to_string()
}

fn read_cpu_model() -> String {
    #[cfg(target_os = "linux")]
    if let Ok(content) = std::fs::read_to_string("/proc/cpuinfo") {
        for line in content.lines() {
            if line.starts_with("model name") {
                if let Some(val) = line.splitn(2, ':').nth(1) {
                    return val.trim().to_string();
                }
            }
        }
    }

    #[cfg(target_os = "macos")]
    if let Ok(out) = std::process::Command::new("sysctl")
        .args(["-n", "machdep.cpu.brand_string"])
        .output()
    {
        if out.status.success() {
            return String::from_utf8_lossy(&out.stdout).trim().to_string();
        }
    }

    "unknown".to_string()
}

fn read_memory_total_kb() -> u64 {
    #[cfg(target_os = "linux")]
    if let Ok(content) = std::fs::read_to_string("/proc/meminfo") {
        for line in content.lines() {
            if line.starts_with("MemTotal:") {
                let kb: u64 = line
                    .split_whitespace()
                    .nth(1)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);
                return kb;
            }
        }
    }

    #[cfg(target_os = "macos")]
    if let Ok(out) = std::process::Command::new("sysctl")
        .args(["-n", "hw.memsize"])
        .output()
    {
        if out.status.success() {
            let bytes: u64 = String::from_utf8_lossy(&out.stdout)
                .trim()
                .parse()
                .unwrap_or(0);
            return bytes / 1024;
        }
    }

    0
}
