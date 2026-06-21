// server/src/node_hardware_probe.rs

use homecli_proto::NodeHardwareProfile;
use std::{collections::HashSet, process::Command};

pub(crate) fn collect_hardware_profile() -> NodeHardwareProfile {
    let mut profile = NodeHardwareProfile {
        os: Some(std::env::consts::OS.to_string()),
        arch: Some(std::env::consts::ARCH.to_string()),
        cpu_brand: detect_cpu_brand(),
        cpu_cores: std::thread::available_parallelism()
            .ok()
            .map(|cores| cores.get() as u32),
        memory_total_bytes: detect_memory_total_bytes(),
        gpu_names: detect_gpu_names(),
        gpu_memory_total_bytes: detect_gpu_memory_total_bytes(),
        disk_free_bytes: None,
    };
    profile.gpu_names = clean_list(profile.gpu_names, 6, 80);
    profile
}

fn clean_list(values: Vec<String>, limit: usize, max_len: usize) -> Vec<String> {
    let mut seen = HashSet::new();
    values
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .map(|value| {
            if value.chars().count() > max_len {
                value.chars().take(max_len).collect::<String>()
            } else {
                value
            }
        })
        .filter(|value| seen.insert(value.to_lowercase()))
        .take(limit)
        .collect()
}

fn detect_cpu_brand() -> Option<String> {
    if cfg!(windows) {
        run_command(
            "powershell",
            &[
                "-NoProfile",
                "-Command",
                "(Get-CimInstance Win32_Processor | Select-Object -First 1 -ExpandProperty Name)",
            ],
        )
        .and_then(first_nonempty_line)
    } else {
        std::fs::read_to_string("/proc/cpuinfo")
            .ok()
            .and_then(|content| {
                content.lines().find_map(|line| {
                    line.split_once(':').and_then(|(key, value)| {
                        if key.trim().eq_ignore_ascii_case("model name") {
                            Some(value.trim().to_string())
                        } else {
                            None
                        }
                    })
                })
            })
    }
}

fn detect_memory_total_bytes() -> Option<u64> {
    if cfg!(windows) {
        run_command(
            "powershell",
            &[
                "-NoProfile",
                "-Command",
                "(Get-CimInstance Win32_ComputerSystem).TotalPhysicalMemory",
            ],
        )
        .and_then(|output| parse_first_u64(&output))
    } else {
        std::fs::read_to_string("/proc/meminfo")
            .ok()
            .and_then(|content| {
                content.lines().find_map(|line| {
                    let (key, rest) = line.split_once(':')?;
                    if key.trim() != "MemTotal" {
                        return None;
                    }
                    let kb = rest.split_whitespace().next()?.parse::<u64>().ok()?;
                    kb.checked_mul(1024)
                })
            })
    }
}

fn detect_gpu_names() -> Vec<String> {
    if cfg!(windows) {
        return run_command(
            "powershell",
            &[
                "-NoProfile",
                "-Command",
                "Get-CimInstance Win32_VideoController | Select-Object -ExpandProperty Name",
            ],
        )
        .map(|output| {
            output
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .map(ToOwned::to_owned)
                .collect()
        })
        .unwrap_or_default();
    }

    if let Some(output) = run_command(
        "nvidia-smi",
        &["--query-gpu=name", "--format=csv,noheader,nounits"],
    ) {
        let names = output
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>();
        if !names.is_empty() {
            return names;
        }
    }

    run_command("lspci", &[])
        .map(|output| {
            output
                .lines()
                .filter(|line| {
                    let lower = line.to_ascii_lowercase();
                    lower.contains("vga") || lower.contains("3d controller")
                })
                .map(|line| {
                    line.split_once(':')
                        .map(|(_, value)| value.trim().to_string())
                        .unwrap_or_else(|| line.trim().to_string())
                })
                .collect()
        })
        .unwrap_or_default()
}

fn detect_gpu_memory_total_bytes() -> Option<u64> {
    if cfg!(windows) {
        return run_command(
            "powershell",
            &[
                "-NoProfile",
                "-Command",
                "Get-CimInstance Win32_VideoController | ForEach-Object { $_.AdapterRAM }",
            ],
        )
        .and_then(|output| {
            let total = output
                .lines()
                .filter_map(|line| line.trim().parse::<u64>().ok())
                .filter(|value| *value > 0)
                .sum::<u64>();
            (total > 0).then_some(total)
        });
    }

    run_command(
        "nvidia-smi",
        &["--query-gpu=memory.total", "--format=csv,noheader,nounits"],
    )
    .and_then(|output| {
        let mib = output
            .lines()
            .filter_map(|line| line.trim().parse::<u64>().ok())
            .sum::<u64>();
        mib.checked_mul(1024 * 1024).filter(|value| *value > 0)
    })
}

fn run_command(program: &str, args: &[&str]) -> Option<String> {
    let mut command = Command::new(program);
    command.args(args);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
        command.creation_flags(CREATE_NO_WINDOW | CREATE_NEW_PROCESS_GROUP);
    }
    let output = command.output().ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout).ok()
}

fn first_nonempty_line(output: String) -> Option<String> {
    output
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(ToOwned::to_owned)
}

fn parse_first_u64(output: &str) -> Option<u64> {
    output
        .lines()
        .find_map(|line| line.trim().parse::<u64>().ok())
}
