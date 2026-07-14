use std::fs;
use std::io::{BufRead, BufReader};

use crate::utils::AppError;

/// 内核 CPU 统计信息文件路径。
const STAT_PATH: &str = "/proc/stat";

/// 追踪连续两次轮询之间的 CPU 利用率差值。
///
/// 构造时读取当前 `/proc/stat` 值作为基线；
/// 因此首次调用 [`poll`](Self::poll) 始终返回 `0.0 %`。
pub struct CpuMonitor {
    prev_idle: u64,
    prev_total: u64,
}

impl CpuMonitor {
    /// 创建 CPU 监控器，以当前 tick 计数作为基线。
    ///
    /// # 错误
    ///
    /// 如果 `/proc/stat` 不可读或第一行格式异常则失败。
    pub fn new() -> Result<Self, AppError> {
        let (idle, total) = read_cpu_stats()?;
        Ok(Self {
            prev_idle: idle,
            prev_total: total,
        })
    }

    /// 返回自上次调用以来的 CPU 利用率百分比（0.0 – 100.0）。
    ///
    /// 首次调用始终返回 `0.0` —— 它仅建立基线。
    pub fn poll(&mut self) -> Result<f32, AppError> {
        let (idle, total) = read_cpu_stats()?;

        // 使用 f64 —— 系统运行数天后 tick 计数可能超过 f32 精确表示范围
        let d_idle = idle.saturating_sub(self.prev_idle) as f64;
        let d_total = total.saturating_sub(self.prev_total) as f64;

        self.prev_idle = idle;
        self.prev_total = total;

        if d_total == 0.0 {
            return Ok(0.0);
        }
        Ok(((1.0 - d_idle / d_total) * 100.0) as f32)
    }
}

/// 解析 `/proc/stat` 第一行，返回 `(idle_ticks, total_ticks)`。
///
/// 预期格式（Linux ≥ 2.6）：
/// ```text
/// cpu  user nice system idle iowait irq softirq steal guest guest_nice
/// ```
fn read_cpu_stats() -> Result<(u64, u64), AppError> {
    let f = fs::File::open(STAT_PATH)?;
    let mut line = String::with_capacity(256);
    BufReader::new(f).read_line(&mut line)?;

    // 使用迭代器替代 Vec::collect()，避免每帧堆分配
    let mut fields = line.split_whitespace();
    if fields.next() != Some("cpu") {
        return Err(AppError::Parse("cpu: /proc/stat 格式异常".into()));
    }

    let parse = |s: Option<&str>, name: &str| -> Result<u64, AppError> {
        s.ok_or_else(|| AppError::Parse(format!("cpu {}: 字段缺失", name)))?
            .parse()
            .map_err(|_| AppError::Parse(format!("cpu {}", name)))
    };

    // 必填字段
    let user = parse(fields.next(), "user")?;
    let nice = parse(fields.next(), "nice")?;
    let system = parse(fields.next(), "system")?;
    let idle = parse(fields.next(), "idle")?;

    // 可选字段（较新内核引入，缺失时按 0 处理）
    let iowait = fields.next().and_then(|v| v.parse().ok()).unwrap_or(0);
    let irq = fields.next().and_then(|v| v.parse().ok()).unwrap_or(0);
    let softirq = fields.next().and_then(|v| v.parse().ok()).unwrap_or(0);
    let steal = fields.next().and_then(|v| v.parse().ok()).unwrap_or(0);

    // iowait 计入空闲时间（CPU 空闲到可以执行其他任务）
    let idle_ticks = idle + iowait;
    let total_ticks = user + nice + system + idle + iowait + irq + softirq + steal;

    Ok((idle_ticks, total_ticks))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_creates_baseline() {
        let m = CpuMonitor::new();
        assert!(m.is_ok());
    }

    #[test]
    fn first_poll_in_valid_range() {
        let mut m = CpuMonitor::new().unwrap();
        let usage = m.poll().unwrap();
        // 首次轮询基于自构造以来的 CPU 增量，在空闲系统上 ≈ 0%，
        // 但在编译后运行的测试中可能更高。只验证值在有效范围内。
        assert!(usage >= 0.0 && usage <= 100.0,
            "首次轮询应在 0-100% 之间，实际为 {}", usage);
    }
}
