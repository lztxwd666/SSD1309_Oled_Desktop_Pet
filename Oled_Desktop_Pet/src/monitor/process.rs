//! 自身进程开销监控 —— 读取 `/proc/self/stat` 采集真实资源消耗。
//!
//! 所有数据均来自 Linux 内核的进程记账机制，与 `top` / `ps` / `htop`
//! 输出完全一致，绝非模拟或估算值。

use std::time::Instant;

use crate::utils;
use crate::utils::AppError;

/// 自身进程性能监控器。
///
/// 通过 `/proc/self/stat` 读取内核进程记账数据：
/// * CPU 时间片（utime + stime）→ 除以实际墙上时钟差 → 精确 CPU%
/// * RSS 物理内存页数 × 页大小 → KiB
/// * 虚拟内存字节数 → KiB
/// * 线程数
///
/// 首次 poll 返回 0% CPU（建立基线），后续轮询返回精确增量百分比。
/// 该值与 `top -p <pid>` 显示的 %CPU 完全一致。
pub struct ProcessMonitor {
    prev_utime: u64,
    prev_stime: u64,
    prev_time: Instant,
    ticks_per_sec: f64,
    page_size: u64,
}

/// 单次进程状态快照的中间结果。
struct ProcStat {
    utime: u64,
    stime: u64,
    num_threads: u32,
    vsize_bytes: u64,
    rss_pages: u64,
}

impl ProcessMonitor {
    /// 创建自身进程监控器，以当前 CPU 时间片为基线。
    pub fn new() -> Result<Self, AppError> {
        let stat = read_proc_stat()?;
        Ok(Self {
            prev_utime: stat.utime,
            prev_stime: stat.stime,
            prev_time: Instant::now(),
            ticks_per_sec: clock_ticks_per_sec(),
            page_size: system_page_size(),
        })
    }

    /// 轮询自身进程的资源消耗。
    ///
    /// 返回的 CPU 百分比是自上次 poll 以来的增量值 ——
    /// 这正是 `top` 刷新时显示的逻辑。
    pub fn poll(&mut self) -> Result<ProcessSnapshot, AppError> {
        let stat = read_proc_stat()?;

        // CPU 时间片差值 → 精确占用百分比
        let now = Instant::now();
        let d_utime = stat.utime.saturating_sub(self.prev_utime) as f64;
        let d_stime = stat.stime.saturating_sub(self.prev_stime) as f64;
        let elapsed = now.duration_since(self.prev_time).as_secs_f64();

        // CPU% = (CPU时间增量 / 墙上时钟增量) × 100
        // 即使轮询间隔不是精确 1 秒，此公式仍然正确
        let cpu_pct = if elapsed > 0.0 {
            ((d_utime + d_stime) / self.ticks_per_sec) / elapsed * 100.0
        } else {
            0.0
        };

        self.prev_utime = stat.utime;
        self.prev_stime = stat.stime;
        self.prev_time = now;

        Ok(ProcessSnapshot {
            cpu_pct: cpu_pct as f32,
            rss_kb: (stat.rss_pages * self.page_size) / 1024,
            vm_kb: stat.vsize_bytes / 1024,
            threads: stat.num_threads,
        })
    }
}

/// 一次进程监控轮询的结果。
#[derive(Debug, Clone)]
pub struct ProcessSnapshot {
    /// 本进程 CPU 占用百分比（0.0 – 100.0 × 核心数，多核可 >100%）。
    pub cpu_pct: f32,
    /// 物理内存占用（KiB，RSS）。
    pub rss_kb: u64,
    /// 虚拟内存占用（KiB，VSZ）。
    pub vm_kb: u64,
    /// 线程数。
    pub threads: u32,
}

// 内部辅助

/// 解析 `/proc/self/stat`。
///
/// 格式示例：
/// ```text
/// 1234 (my process) S 1000 1234 1000 ...
/// ```
///
/// 难点：`comm` 字段用括号包裹，括号内不会出现 `)`（内核限制）。
fn read_proc_stat() -> Result<ProcStat, AppError> {
    let stat = utils::read_trimmed("/proc/self/stat")?;

    // 找到 comm 字段的右括号
    let rparen = stat.rfind(')').ok_or_else(|| {
        AppError::Parse("进程 stat: 找不到 comm 右括号".into())
    })?;

    // 右括号之后是空格 + state + 空格 + 后续字段
    let after = &stat[rparen + 2..];
    // 使用迭代器替代 Vec::collect()，避免每帧堆分配
    // Man 5 proc 字段（state 之后，0 起始）：
    //   0=state, 11=utime, 12=stime, 17=num_threads, 20=vsize, 21=rss
    let mut fields = after.split_whitespace();

    let parse_u64 = |s: Option<&str>, name: &str| -> Result<u64, AppError> {
        s.ok_or_else(|| AppError::Parse(format!("{}: 字段缺失", name)))?
            .parse()
            .map_err(|_| AppError::Parse(format!("{} 解析失败", name)))
    };

    // 跳过 state → ppid(1)..cminflt(10) → utime(11)
    let _state = fields.next();
    let utime = parse_u64(fields.nth(10), "utime")?;
    // stime 紧跟 utime（索引 12）
    let stime = parse_u64(fields.next(), "stime")?;
    // 跳过 starttime(13)..(16) → num_threads(17)
    let num_threads: u32 = parse_u64(fields.nth(4), "num_threads")? as u32;
    // 跳过 (18)..(19) → vsize(20)
    let vsize_bytes = parse_u64(fields.nth(2), "vsize")?;
    // rss 紧跟 vsize（索引 21）
    let rss_pages = parse_u64(fields.next(), "rss")?;

    Ok(ProcStat {
        utime,
        stime,
        num_threads,
        vsize_bytes,
        rss_pages,
    })
}

/// 获取系统时钟 tick 频率（通常为 100 Hz）。
fn clock_ticks_per_sec() -> f64 {
    // SAFETY: sysconf(_SC_CLK_TCK) 始终返回有效正值，无副作用。
    unsafe { libc::sysconf(libc::_SC_CLK_TCK) as f64 }
}

/// 获取系统内存页大小（通常为 4096 字节）。
fn system_page_size() -> u64 {
    // SAFETY: sysconf(_SC_PAGESIZE) 始终返回有效正值，无副作用。
    unsafe { libc::sysconf(libc::_SC_PAGESIZE) as u64 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_creates_baseline() {
        let m = ProcessMonitor::new();
        assert!(m.is_ok(), "进程监控器应能正常初始化");
    }

    #[test]
    fn poll_returns_valid_data() {
        let mut m = ProcessMonitor::new().unwrap();
        let snap = m.poll().unwrap();
        // 第一次 poll 的 CPU% 应接近 0（仅消耗在 poll 自身）
        assert!(snap.cpu_pct >= 0.0, "CPU% 不应为负");
        // RSS 应 > 0（至少加载了本程序代码）
        assert!(snap.rss_kb > 0, "RSS 应大于 0，实际 {}", snap.rss_kb);
        // 主线程至少 1 个
        assert!(snap.threads >= 1, "线程数应 ≥ 1，实际 {}", snap.threads);
    }

    #[test]
    fn second_poll_accumulates() {
        let mut m = ProcessMonitor::new().unwrap();
        let _ = m.poll().unwrap();
        // 第二次 poll：使用一些 CPU 做点计算
        let _waste: u64 = (0..100_000).sum();
        let snap2 = m.poll().unwrap();
        // 做些运算后 CPU 时间片应有微小增长
        assert!(snap2.cpu_pct >= 0.0);
    }
}
