//! Type-C 接口事件源 —— 监控 /sys/class/typec/ 端口状态变化。
//!
//! 如果系统没有 Type-C 控制器（如 Pi 5 仅供电无数据角色），
//! 自动静默降级为无操作。

use std::collections::HashSet;
use std::fs;

use super::{Notification, Notifier, NotifyKind};

pub struct TypeCMonitor {
    prev_ports: HashSet<String>,
}

impl TypeCMonitor {
    pub fn new() -> Self {
        Self { prev_ports: list_typec_ports().unwrap_or_default() }
    }
}

impl super::EventSource for TypeCMonitor {
    fn poll(&mut self, notifier: &mut Notifier) {
        let current = match list_typec_ports() {
            Some(d) => d,
            None => return, // /sys/class/typec/ 不存在，静默跳过
        };

        for name in current.difference(&self.prev_ports) {
            notifier.push(Notification::new(
                NotifyKind::TypeCInsert,
                format!("TypeC {name} in"),
            ));
        }
        for name in self.prev_ports.difference(&current) {
            notifier.push(Notification::new(
                NotifyKind::TypeCRemove,
                format!("TypeC {name} out"),
            ));
        }

        self.prev_ports = current;
    }
}

/// 列出 /sys/class/typec/ 中的端口条目（如 port0, port1）。
/// 目录不存在时返回 None。
fn list_typec_ports() -> Option<HashSet<String>> {
    let entries = fs::read_dir("/sys/class/typec").ok()?;
    let mut ports = HashSet::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with("port") {
            ports.insert(name);
        }
    }
    Some(ports)
}
