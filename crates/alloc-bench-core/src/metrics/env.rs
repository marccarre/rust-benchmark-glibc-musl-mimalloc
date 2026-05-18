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
    // WR-08: '/proc/cpuinfo' uses 'model name' on x86 but 'Processor' /
    // 'CPU implementer' / 'CPU part' / 'Model' on aarch64 and other
    // architectures. Fall back through the most-specific keys first so we
    // get a useful label on every Linux arch the project supports
    // (including aarch64-linux-* per STACK.md).
    #[cfg(target_os = "linux")]
    if let Ok(content) = std::fs::read_to_string("/proc/cpuinfo") {
        // Order matters: 'model name' is x86-specific; 'Model' / 'Processor'
        // appear on aarch64; 'CPU implementer' is the last-resort vendor ID.
        const KEYS: &[&str] = &["model name", "Model", "Processor", "CPU implementer"];
        for key in KEYS {
            for line in content.lines() {
                if let Some((k, v)) = line.split_once(':') {
                    if k.trim() == *key {
                        let val = v.trim();
                        if !val.is_empty() {
                            return val.to_string();
                        }
                    }
                }
            }
        }
        // Embedded boards sometimes only populate /sys for the device tree.
        if let Ok(compat) =
            std::fs::read_to_string("/sys/devices/system/cpu/cpu0/of_node/compatible")
        {
            // The file is a NUL-separated list of strings; first entry is
            // the most-specific compatible string.
            if let Some(first) = compat.split('\0').find(|s| !s.is_empty()) {
                return first.trim().to_string();
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
