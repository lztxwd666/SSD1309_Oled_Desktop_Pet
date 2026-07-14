//! 系统资源告警事件源 —— 基于 SystemInfo 的阈值触发。
//!
//! 全部告警带滞回，避免边界抖动。磁盘使用率通过 statvfs 获取。

use crate::model::SystemInfo;

use super::{Notification, Notifier, NotifyKind};

/// 告警阈值配置。
#[derive(Debug, Clone)]
pub struct AlertConfig {
    pub temp_high: f32,
    pub temp_safe: f32,
    pub mem_high: f32,
    pub mem_safe: f32,
    pub disk_high: f32,
    pub disk_safe: f32,
}

impl Default for AlertConfig {
    fn default() -> Self {
        Self {
            temp_high: 80.0, temp_safe: 75.0,
            mem_high: 90.0, mem_safe: 85.0,
            disk_high: 90.0, disk_safe: 85.0,
        }
    }
}

pub struct SystemAlerts {
    temp_fired: bool,
    mem_fired: bool,
    disk_fired: bool,
    config: AlertConfig,
}

impl SystemAlerts {
    pub fn new(config: AlertConfig) -> Self {
        Self { temp_fired: false, mem_fired: false, disk_fired: false, config }
    }

    pub fn check(&mut self, info: &SystemInfo, notifier: &mut Notifier) {
        if info.cpu_temp_celsius >= self.config.temp_high && !self.temp_fired {
            notifier.push(Notification::new(
                NotifyKind::TempAlert,
                format!("Hot {:.0}°C", info.cpu_temp_celsius),
            ));
            self.temp_fired = true;
        } else if info.cpu_temp_celsius <= self.config.temp_safe {
            self.temp_fired = false;
        }

        if info.mem_usage_pct >= self.config.mem_high && !self.mem_fired {
            notifier.push(Notification::new(
                NotifyKind::MemAlert,
                format!("RAM {:.0}%", info.mem_usage_pct),
            ));
            self.mem_fired = true;
        } else if info.mem_usage_pct <= self.config.mem_safe {
            self.mem_fired = false;
        }

        let disk_pct = crate::utils::disk_usage_pct();
        if !self.disk_fired {
            if let Some(pct) = disk_pct
                && pct >= self.config.disk_high {
                    notifier.push(Notification::new(
                        NotifyKind::DiskAlert,
                        format!("Disk {:.0}%", pct),
                    ));
                    self.disk_fired = true;
                }
        } else if disk_pct.is_none_or(|p| p <= self.config.disk_safe) {
            self.disk_fired = false;
        }
    }
}
