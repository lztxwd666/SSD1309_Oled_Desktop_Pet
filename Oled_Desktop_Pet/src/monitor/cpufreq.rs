//! CPU 频率监控 —— 读取当前频率和最大频率，检测降频。

use std::fs;

use crate::utils;

const CUR_FREQ: &str = "/sys/devices/system/cpu/cpu0/cpufreq/scaling_cur_freq";
const MAX_FREQ: &str = "/sys/devices/system/cpu/cpu0/cpufreq/scaling_max_freq";

pub struct CpuFreqMonitor;

impl CpuFreqMonitor {
    pub fn new() -> Self { Self }

    /// 返回 (当前 GHz, 最大 GHz)。
    pub fn poll(&self) -> (f32, f32) {
        let cur = read_khz(CUR_FREQ);
        let max = read_khz(MAX_FREQ);
        (cur as f32 / 1_000_000.0, max as f32 / 1_000_000.0)
    }
}

fn read_khz(path: &str) -> u64 {
    fs::read_to_string(path).ok()
        .and_then(|s| utils::parse_u64_prefix(&s))
        .unwrap_or(0)
}
