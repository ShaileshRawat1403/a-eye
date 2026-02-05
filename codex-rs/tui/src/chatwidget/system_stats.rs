use std::process::Command;
use std::sync::Mutex;
use std::sync::OnceLock;
use std::time::Duration;
use std::time::Instant;

use sysinfo::CpuRefreshKind;
use sysinfo::MemoryRefreshKind;
use sysinfo::RefreshKind;
use sysinfo::System;

const MEBIBYTE: f64 = 1024.0 * 1024.0;
const GIBIBYTE: f64 = 1024.0 * 1024.0 * 1024.0;
const GPU_REFRESH_INTERVAL: Duration = Duration::from_secs(4);

#[derive(Clone, Debug, Default)]
pub(crate) struct SystemStatsSnapshot {
    pub(crate) cpu_percent: Option<f32>,
    pub(crate) memory_percent: Option<f32>,
    #[allow(dead_code)]
    pub(crate) memory_used_gib: Option<f32>,
    #[allow(dead_code)]
    pub(crate) memory_total_gib: Option<f32>,
    pub(crate) gpu_percent: Option<f32>,
    #[allow(dead_code)]
    pub(crate) gpu_source: Option<&'static str>,
}

#[derive(Debug)]
struct SystemStatsSampler {
    system: System,
    last_gpu_probe: Option<Instant>,
    cached_gpu_percent: Option<f32>,
    cached_gpu_source: Option<&'static str>,
}

impl SystemStatsSampler {
    fn new() -> Self {
        let mut system = System::new_with_specifics(
            RefreshKind::nothing()
                .with_cpu(CpuRefreshKind::everything())
                .with_memory(MemoryRefreshKind::everything()),
        );
        system.refresh_cpu_usage();
        system.refresh_memory();
        Self {
            system,
            last_gpu_probe: None,
            cached_gpu_percent: None,
            cached_gpu_source: None,
        }
    }

    fn sample(&mut self) -> SystemStatsSnapshot {
        self.system.refresh_cpu_usage();
        self.system.refresh_memory();

        let total_memory_mib = self.system.total_memory() as f64 / MEBIBYTE;
        let used_memory_mib = self.system.used_memory() as f64 / MEBIBYTE;
        let memory_percent = (total_memory_mib > 0.0)
            .then_some(((used_memory_mib / total_memory_mib) * 100.0) as f32);
        let memory_used_gib = Some((used_memory_mib * MEBIBYTE / GIBIBYTE) as f32);
        let memory_total_gib = Some((total_memory_mib * MEBIBYTE / GIBIBYTE) as f32);

        let should_probe_gpu = self
            .last_gpu_probe
            .is_none_or(|last_probe| last_probe.elapsed() >= GPU_REFRESH_INTERVAL);
        if should_probe_gpu {
            if let Some((gpu_percent, source)) = probe_gpu_percent() {
                self.cached_gpu_percent = Some(gpu_percent);
                self.cached_gpu_source = Some(source);
            } else {
                self.cached_gpu_percent = None;
                self.cached_gpu_source = None;
            }
            self.last_gpu_probe = Some(Instant::now());
        }

        SystemStatsSnapshot {
            cpu_percent: Some(self.system.global_cpu_usage()),
            memory_percent,
            memory_used_gib,
            memory_total_gib,
            gpu_percent: self.cached_gpu_percent,
            gpu_source: self.cached_gpu_source,
        }
    }
}

fn probe_gpu_percent() -> Option<(f32, &'static str)> {
    probe_nvidia_gpu_percent()
        .map(|percent| (percent, "nvidia-smi"))
        .or_else(|| probe_rocm_gpu_percent().map(|percent| (percent, "rocm-smi")))
        .or_else(|| probe_macos_gpu_percent().map(|percent| (percent, "ioreg")))
}

fn probe_nvidia_gpu_percent() -> Option<f32> {
    let output = Command::new("nvidia-smi")
        .args([
            "--query-gpu=utilization.gpu",
            "--format=csv,noheader,nounits",
        ])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let raw = String::from_utf8(output.stdout).ok()?;
    average_percent(
        raw.lines()
            .filter_map(|line| line.trim().parse::<f32>().ok()),
    )
}

fn probe_rocm_gpu_percent() -> Option<f32> {
    let output = Command::new("rocm-smi").args(["--showuse"]).output().ok()?;
    if !output.status.success() {
        return None;
    }

    parse_rocm_gpu_showuse_output(&String::from_utf8(output.stdout).ok()?)
}

fn probe_macos_gpu_percent() -> Option<f32> {
    if !cfg!(target_os = "macos") {
        return None;
    }

    let output = Command::new("ioreg")
        .args(["-r", "-d", "1", "-w", "0", "-c", "IOAccelerator"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    parse_device_utilization_percent(&String::from_utf8(output.stdout).ok()?)
}

fn parse_device_utilization_percent(raw: &str) -> Option<f32> {
    let marker = "\"Device Utilization %\"=";
    let start = raw.find(marker)?;
    let value_start = start + marker.len();
    let tail = raw.get(value_start..)?.trim_start();

    let digits: String = tail
        .chars()
        .take_while(|char| char.is_ascii_digit() || *char == '.')
        .collect();
    (!digits.is_empty())
        .then_some(digits)
        .and_then(|value| value.parse::<f32>().ok())
}

fn parse_rocm_gpu_showuse_output(raw: &str) -> Option<f32> {
    average_percent(
        raw.lines()
            .filter(|line| line.contains("GPU use") || line.contains("GFX Activity"))
            .filter_map(parse_percent_after_last_colon),
    )
}

fn parse_percent_after_last_colon(line: &str) -> Option<f32> {
    let value = line.rsplit(':').next()?.trim();
    let digits: String = value
        .chars()
        .take_while(|char| char.is_ascii_digit() || *char == '.')
        .collect();
    (!digits.is_empty())
        .then_some(digits)
        .and_then(|raw| raw.parse::<f32>().ok())
}

fn average_percent(values: impl Iterator<Item = f32>) -> Option<f32> {
    let mut count = 0_u32;
    let mut sum = 0.0_f32;
    for value in values {
        if !value.is_finite() {
            continue;
        }
        sum += value.clamp(0.0, 100.0);
        count += 1;
    }
    (count > 0).then_some(sum / count as f32)
}

static SYSTEM_STATS_SAMPLER: OnceLock<Mutex<SystemStatsSampler>> = OnceLock::new();

pub(crate) fn sample_system_stats() -> SystemStatsSnapshot {
    if cfg!(test) {
        return SystemStatsSnapshot {
            cpu_percent: Some(42.0),
            memory_percent: Some(58.0),
            memory_used_gib: Some(5.5),
            memory_total_gib: Some(8.0),
            gpu_percent: Some(23.0),
            gpu_source: Some("test-gpu"),
        };
    }

    let sampler = SYSTEM_STATS_SAMPLER.get_or_init(|| Mutex::new(SystemStatsSampler::new()));
    match sampler.lock() {
        Ok(mut guard) => guard.sample(),
        Err(_) => SystemStatsSnapshot::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::parse_device_utilization_percent;
    use super::parse_rocm_gpu_showuse_output;
    use pretty_assertions::assert_eq;

    #[test]
    fn parses_macos_ioreg_device_utilization_percent() {
        let raw =
            r#"PerformanceStatistics" = {"Device Utilization %"=34,"Renderer Utilization %"=33}"#;
        assert_eq!(parse_device_utilization_percent(raw), Some(34.0));
    }

    #[test]
    fn returns_none_without_device_utilization_percent_marker() {
        let raw = r#"PerformanceStatistics" = {"Renderer Utilization %"=33}"#;
        assert_eq!(parse_device_utilization_percent(raw), None);
    }

    #[test]
    fn parses_rocm_gpu_showuse_percent() {
        let raw = "\
========================ROCm System Management Interface========================
GPU[0]          : GPU use (%): 43
GPU[1]          : GPU use (%): 17
================================================================================";
        assert_eq!(parse_rocm_gpu_showuse_output(raw), Some(30.0));
    }

    #[test]
    fn parses_rocm_gfx_activity_percent() {
        let raw = "GPU[0]          : GFX Activity: 61.5%";
        assert_eq!(parse_rocm_gpu_showuse_output(raw), Some(61.5));
    }
}
