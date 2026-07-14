use std::fs;
use std::io::{BufRead, BufReader};

use crate::utils;
use crate::utils::AppError;

/// 内核内存信息文件路径。
const MEMINFO_PATH: &str = "/proc/meminfo";

/// 从 `/proc/meminfo` 读取系统内存统计。
///
/// 总内存（`MemTotal`）在构造时读取一次并缓存 —— 它在运行时不会改变；
/// *已用量* 和 *百分比* 每次轮询时根据 `MemAvailable` 重新计算。
pub struct MemoryMonitor {
    total_kb: u64,
}

impl MemoryMonitor {
    /// 创建内存监控器。永久缓存 `MemTotal`。
    ///
    /// 如果 `/proc/meminfo` 在构造时不可读，`total_kb` 回退到 `0`，
    /// 后续所有轮询均返回 `0`。
    pub fn new() -> Self {
        let total_kb = read_field("MemTotal:").unwrap_or(0);
        Self { total_kb }
    }

    /// 系统总内存（KiB）。
    #[inline]
    pub fn total_kb(&self) -> u64 {
        self.total_kb
    }

    /// 估算已用内存（total − available），单位 KiB。
    ///
    /// 使用 `MemAvailable`（内核估算的可回收内存），
    /// 它比单独使用 `MemFree` 更能反映"实际可用"的情况。
    pub fn used_kb(&self) -> Result<u64, AppError> {
        let available = read_field("MemAvailable:")?;
        Ok(self.total_kb.saturating_sub(available.min(self.total_kb)))
    }

    /// 内存使用百分比（0.0 – 100.0）。接受已计算的 used_kb 避免重复读取 /proc。
    pub fn percent_from(&self, used_kb: u64) -> f32 {
        if self.total_kb == 0 { 0.0 }
        else { used_kb as f32 / self.total_kb as f32 * 100.0 }
    }

    /// 内存使用百分比（0.0 – 100.0）。单独调用时需读取 /proc。
    pub fn percent(&self) -> Result<f32, AppError> {
        Ok(self.percent_from(self.used_kb()?))
    }
}

/// 扫描 `/proc/meminfo`，找到以 `field` 开头的行并返回数值。
///
/// 标准 meminfo 字段的数值单位始终为 KiB。
fn read_field(field: &str) -> Result<u64, AppError> {
    let f = fs::File::open(MEMINFO_PATH)?;
    for line in BufReader::new(f).lines() {
        let line = line?;
        if let Some(value_str) = line.strip_prefix(field) {
            return utils::parse_u64_prefix(value_str)
                .ok_or_else(|| AppError::Parse(format!("meminfo: {}", line)));
        }
    }
    Err(AppError::NotFound(format!("meminfo 字段: {}", field)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_always_succeeds() {
        let m = MemoryMonitor::new();
        // 真实 Linux 上 total_kb > 0；容器 / CI 中可能为 0
        // 无论哪种情况构造都不应 panic
        let _ = m.total_kb();
    }

    #[test]
    fn used_and_percent_do_not_panic() {
        let m = MemoryMonitor::new();
        let _ = m.used_kb();
        let _ = m.percent();
    }
}
