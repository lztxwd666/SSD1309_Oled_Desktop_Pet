//! 网络接口状态事件源 —— 监控接口的启用/禁用变化。
//!
//! 接口名映射为友好名称：eth0/end0→"有线网络"，wlan0→"WiFi"。

use std::collections::HashMap;
use std::fs;

use super::{Notification, Notifier, NotifyKind};

pub struct IfaceMonitor {
    prev_states: HashMap<String, bool>,
    /// 缓存 `/sys/class/net/{name}/operstate` 路径，避免每帧 format!()。
    path_cache: HashMap<String, String>,
}

impl IfaceMonitor {
    pub fn new() -> Self {
        let mut this = Self {
            prev_states: HashMap::new(),
            path_cache: HashMap::new(),
        };
        this.prev_states = this.read_iface_states();
        this
    }
}

impl super::EventSource for IfaceMonitor {
    fn poll(&mut self, notifier: &mut Notifier) {
        let current = self.read_iface_states();

        // 收集本轮所有变化，合并为一条通知（预分配容量避免重新分配）
        let mut ups: Vec<&str> = Vec::with_capacity(4);
        let mut downs: Vec<&str> = Vec::with_capacity(4);

        for (name, is_up) in &current {
            match self.prev_states.get(name) {
                None if *is_up => ups.push(name),
                Some(was_up) if *was_up != *is_up => {
                    if *is_up {
                        ups.push(name)
                    } else {
                        downs.push(name)
                    }
                }
                _ => {}
            }
        }
        for name in self.prev_states.keys() {
            if !current.contains_key(name) {
                downs.push(name);
            }
        }

        // 合并推送：多个接口用空格连接
        if !ups.is_empty() {
            notifier.push(Notification::new(
                NotifyKind::IfaceUp,
                format!("{} up", ups.join(" ")),
            ));
        }
        if !downs.is_empty() {
            notifier.push(Notification::new(
                NotifyKind::IfaceDown,
                format!("{} down", downs.join(" ")),
            ));
        }

        self.prev_states = current;
    }
}

impl IfaceMonitor {
    /// 读取所有非 lo 接口的 operstate。
    /// 使用缓存的路径字符串，避免每帧 format!() 分配。
    fn read_iface_states(&mut self) -> HashMap<String, bool> {
        let mut states = HashMap::new();
        let dir = match fs::read_dir("/sys/class/net") {
            Ok(d) => d,
            Err(_) => return states,
        };
        for entry in dir.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name == "lo" {
                continue;
            }
            // 按需缓存路径
            let path = self
                .path_cache
                .entry(name.clone())
                .or_insert_with(|| format!("/sys/class/net/{name}/operstate"));
            let state = fs::read_to_string(path)
                .map(|s| s.trim() == "up")
                .unwrap_or(false);
            states.insert(name, state);
        }
        states
    }
}
