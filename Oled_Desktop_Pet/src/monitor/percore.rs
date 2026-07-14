//! 每核 CPU 利用率 —— 从 /proc/stat 读取 cpu0..cpu3 独立统计。

use std::fs;
use std::io::{BufRead, BufReader};
use crate::utils::AppError;

const STAT_PATH: &str = "/proc/stat";

pub struct PerCoreMonitor {
    prev_idle: [u64; 4],
    prev_total: [u64; 4],
}

impl PerCoreMonitor {
    pub fn new() -> Result<Self, AppError> {
        let cores = read_percore_stats()?;
        let mut prev_idle = [0u64; 4];
        let mut prev_total = [0u64; 4];
        for i in 0..4 {
            prev_idle[i] = cores[i].0;
            prev_total[i] = cores[i].1;
        }
        Ok(Self { prev_idle, prev_total })
    }

    /// 返回 [core0%, core1%, core2%, core3%]。
    pub fn poll(&mut self) -> Result<[f32; 4], AppError> {
        let cores = read_percore_stats()?;
        let mut usage = [0.0f32; 4];
        for i in 0..4 {
            let d_idle = cores[i].0.saturating_sub(self.prev_idle[i]) as f64;
            let d_total = cores[i].1.saturating_sub(self.prev_total[i]) as f64;
            self.prev_idle[i] = cores[i].0;
            self.prev_total[i] = cores[i].1;
            usage[i] = if d_total > 0.0 { ((1.0 - d_idle / d_total) * 100.0) as f32 } else { 0.0 };
        }
        Ok(usage)
    }
}

/// 返回 [(idle, total); 4] 对应 cpu0..cpu3。
fn read_percore_stats() -> Result<[(u64, u64); 4], AppError> {
    let f = fs::File::open(STAT_PATH)?;
    let reader = BufReader::new(f);
    let mut cores = [(0u64, 0u64); 4];

    for line in reader.lines().skip(1) {
        let line = line?;
        if !line.starts_with("cpu") { break; }

        let mut fields = line.split_whitespace();
        let cpu_name = fields.next().unwrap_or("");
        let core_idx: usize = match cpu_name.strip_prefix("cpu") {
            Some(s) => s.parse().unwrap_or(99),
            None => continue,
        };
        if core_idx >= 4 { continue; }

        let mut next_u64 = || fields.next().and_then(|s| s.parse().ok()).unwrap_or(0);
        let user = next_u64();
        let nice = next_u64();
        let system = next_u64();
        let idle = next_u64();
        let iowait = next_u64();
        let irq = next_u64();
        let softirq = next_u64();
        let steal = next_u64();

        let idle_ticks = idle + iowait;
        let total_ticks = user + nice + system + idle + iowait + irq + softirq + steal;
        cores[core_idx] = (idle_ticks, total_ticks);
    }
    Ok(cores)
}
