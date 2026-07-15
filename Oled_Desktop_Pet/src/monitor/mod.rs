//! 系统监控层 —— 从 procfs / sysfs 读取硬件指标。
//!
//! 每个子系统是一个独立结构体。SystemMonitor 持有所有子系统，
//! `poll_all()` 采用尽力而为原则：个别子系统失败时对应字段静默归零。

mod cpu;
mod cpufreq;
mod memory;
mod network;
mod percore;
mod process;
mod thermal;

pub use cpu::CpuMonitor;
pub use cpufreq::CpuFreqMonitor;
pub use memory::MemoryMonitor;
pub use network::NetworkMonitor;
pub use percore::PerCoreMonitor;
pub use process::ProcessMonitor;
pub use thermal::ThermalMonitor;

use crate::model::SystemInfo;
use crate::utils::AppError;

/// 持有所有子系统监控器，协调全系统轮询。
pub struct SystemMonitor {
    thermal: ThermalMonitor,
    cpu: CpuMonitor,
    memory: MemoryMonitor,
    network: NetworkMonitor,
    process: ProcessMonitor,
    percore: PerCoreMonitor,
    cpufreq: CpuFreqMonitor,
    /// 最近一次 poll 的原始频率值（供外部降频判定）
    pub last_cpu_freq: f32,
    pub last_cpu_max: f32,
}

impl SystemMonitor {
    #[allow(dead_code)]
    pub fn new() -> Result<Self, AppError> {
        Self::from_paths(
            "/sys/class/thermal/thermal_zone0/temp".into(),
            vec!["eth0".into(), "end0".into(), "wlan0".into()],
        )
    }

    /// 带可配置路径和接口优先级的构造器。
    pub fn from_paths(thermal_path: String, iface_priority: Vec<String>) -> Result<Self, AppError> {
        Ok(Self {
            thermal: ThermalMonitor::new(thermal_path),
            cpu: CpuMonitor::new()?,
            memory: MemoryMonitor::new(),
            network: NetworkMonitor::new_with_priority(iface_priority)?,
            process: ProcessMonitor::new()?,
            percore: PerCoreMonitor::new()?,
            cpufreq: CpuFreqMonitor::new(),
            last_cpu_freq: 0.0,
            last_cpu_max: 0.0,
        })
    }

    /// 轮询所有子系统。个别失败被静默吸收。
    pub fn poll_all(&mut self) -> SystemInfo {
        let mut info = SystemInfo::default();

        if let Ok(t) = self.thermal.poll() {
            info.cpu_temp_celsius = t;
        }
        if let Ok(pct) = self.cpu.poll() {
            info.cpu_usage_pct = pct;
        }
        info.mem_total_kb = self.memory.total_kb();
        if let Ok(used) = self.memory.used_kb() {
            info.mem_used_kb = used;
            info.mem_usage_pct = self.memory.percent_from(used); // 同次快照，不重读
        }
        if self.network.poll().is_ok() {
            info.net_rx_bytes = self.network.total_rx();
            info.net_tx_bytes = self.network.total_tx();
            info.net_rx_rate_kibs = self.network.rx_rate_kbps();
            info.net_tx_rate_kibs = self.network.tx_rate_kbps();
        }
        if let Ok(snap) = self.process.poll() {
            info.self_cpu_pct = snap.cpu_pct;
            info.self_rss_kb = snap.rss_kb;
            info.self_vm_kb = snap.vm_kb;
            info.self_threads = snap.threads;
        }
        if let Ok(cores) = self.percore.poll() {
            info.per_core_pct = cores;
        }
        let (freq, _max) = self.cpufreq.poll();
        info.cpu_freq_ghz = freq;
        // 存储原始数据供外部配置化降频判定使用
        self.last_cpu_freq = freq;
        self.last_cpu_max = _max;

        info.timestamp = std::time::Instant::now();
        info
    }
}
