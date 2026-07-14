use std::time::Instant;

/// 单次轮询的完整系统状态快照。
///
/// 这是从 monitor 层向上流向 engine / UI 层的**唯一**数据类型。
/// 速率字段（CPU 百分比、网络 KiB/s）基于与上次轮询的差值计算；
/// 首次轮询时返回 `0.0`（尚无基线）。
///
/// 在 aarch64 上约 100 字节 —— 拷贝成本极低。
#[derive(Debug, Clone)]
pub struct SystemInfo {
    // 系统 CPU
    /// CPU 封装温度（摄氏度）。
    pub cpu_temp_celsius: f32,
    /// 系统全局 CPU 利用率百分比（0.0 – 100.0）。
    pub cpu_usage_pct: f32,

    // 系统内存
    /// 系统总内存（KiB）。
    pub mem_total_kb: u64,
    /// 估算已用内存（KiB），计算方式为 `total - available`。
    pub mem_used_kb: u64,
    /// 内存使用百分比（0.0 – 100.0）。
    pub mem_usage_pct: f32,

    // 系统网络
    /// 活跃网络接口的累计接收字节数（启动至今）。
    pub net_rx_bytes: u64,
    /// 活跃网络接口的累计发送字节数（启动至今）。
    pub net_tx_bytes: u64,
    /// 自上次轮询以来的接收速率（KiB/s）。
    pub net_rx_rate_kibs: f32,
    /// 自上次轮询以来的发送速率（KiB/s）。
    pub net_tx_rate_kibs: f32,

    // 自身进程开销
    /// 本进程 CPU 占用百分比（0.0 – 100.0 × 核心数）。
    ///
    /// 来源：`/proc/self/stat` 的 `utime + stime` 增量 / 墙上时钟增量。
    /// 该值与 `top` / `htop` 显示的本进程 CPU% 完全一致。
    pub self_cpu_pct: f32,
    /// 本进程物理内存占用（KiB），即 RSS（Resident Set Size）。
    ///
    /// 来源：`/proc/self/stat` 的 `rss` 字段 × 页大小。
    /// 该值与 `ps -o rss` 或 `top` 的 RES 列完全一致。
    pub self_rss_kb: u64,
    /// 本进程虚拟内存占用（KiB）。
    ///
    /// 来源：`/proc/self/stat` 的 `vsize` 字段。
    /// 该值与 `ps -o vsz` 或 `top` 的 VIRT 列完全一致。
    pub self_vm_kb: u64,
    /// 本进程当前线程数。
    pub self_threads: u32,

    // 频率
    /// 当前 CPU 频率（GHz）。
    pub cpu_freq_ghz: f32,
    /// CPU 降频标志（当前 < 最大 × 70%）。
    pub cpu_throttling: bool,
    /// 温度趋势：1=上升，0=稳定，-1=下降。
    pub temp_trend: i8,
    /// 每核 CPU 利用率 [core0%, core1%, core2%, core3%]。
    pub per_core_pct: [f32; 4],

    /// 快照采集时刻的墙上时钟。
    pub timestamp: Instant,
}

impl Default for SystemInfo {
    fn default() -> Self {
        Self {
            cpu_temp_celsius: 0.0,
            cpu_usage_pct: 0.0,
            mem_total_kb: 0,
            mem_used_kb: 0,
            mem_usage_pct: 0.0,
            net_rx_bytes: 0,
            net_tx_bytes: 0,
            net_rx_rate_kibs: 0.0,
            net_tx_rate_kibs: 0.0,
            self_cpu_pct: 0.0,
            self_rss_kb: 0,
            self_vm_kb: 0,
            self_threads: 0,
            cpu_freq_ghz: 0.0,
            cpu_throttling: false,
            temp_trend: 0,
            per_core_pct: [0.0; 4],
            timestamp: Instant::now(),
        }
    }
}
