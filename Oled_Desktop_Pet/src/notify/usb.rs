//! USB 设备事件源 —— 监控 /sys/bus/usb/devices/ 中设备的新增和移除。
//!
//! 有产品名时显示产品名，无产品名时显示通用描述。

use std::collections::{HashMap, HashSet};
use std::fs;

use super::{Notification, Notifier, NotifyKind};

pub struct UsbMonitor {
    prev_devices: HashSet<String>,
    product_cache: HashMap<String, String>,
    /// 新设备 product 文件尚未就绪时暂存，下次轮询重试。
    pending_names: HashSet<String>,
}

impl UsbMonitor {
    pub fn new() -> Self {
        let devices = list_usb_devices().unwrap_or_default();
        let cache = devices
            .iter()
            .filter_map(|d| usb_product_name(d).map(|p| (d.clone(), p)))
            .collect();
        Self { prev_devices: devices, product_cache: cache, pending_names: HashSet::new() }
    }
}

impl super::EventSource for UsbMonitor {
    fn poll(&mut self, notifier: &mut Notifier) {
        let current = match list_usb_devices() {
            Some(d) => d,
            None => return,
        };

        // 重试之前未读到产品名的设备（retain 直接过滤，无需中间 Vec）
        self.pending_names.retain(|name| {
            if let Some(desc) = usb_product_name(name) {
                self.product_cache.insert(name.clone(), desc.clone());
                notifier.push(Notification::new(NotifyKind::UsbInsert, format!("USB {desc}")));
                false // 已解析，移出 pending
            } else {
                true // 仍然不可用，继续等待
            }
        });

        // 新插入设备
        for name in current.difference(&self.prev_devices) {
            if let Some(desc) = usb_product_name(name) {
                self.product_cache.insert(name.clone(), desc.clone());
                notifier.push(Notification::new(NotifyKind::UsbInsert, format!("USB {desc}")));
            } else {
                // product 文件尚未就绪，延迟到下次轮询
                self.pending_names.insert(name.clone());
            }
        }

        // 拔出设备
        for name in self.prev_devices.difference(&current) {
            let desc = self.product_cache.get(name).cloned().unwrap_or_else(|| "device".into());
            self.product_cache.remove(name);
            self.pending_names.remove(name);
            notifier.push(Notification::new(NotifyKind::UsbRemove, format!("USB {desc} out")));
        }

        self.prev_devices = current;
    }
}

fn list_usb_devices() -> Option<HashSet<String>> {
    let entries = fs::read_dir("/sys/bus/usb/devices").ok()?;
    let mut devices = HashSet::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with("usb") && name[3..].chars().all(|c| c.is_ascii_digit()) {
            continue;
        }
        devices.insert(name);
    }
    Some(devices)
}

fn usb_product_name(device_id: &str) -> Option<String> {
    fs::read_to_string(format!("/sys/bus/usb/devices/{}/product", device_id))
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}
