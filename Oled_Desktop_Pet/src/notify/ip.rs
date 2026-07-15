//! IP 地址通知源 —— 接口感知，新接口立即响应。

use std::collections::HashSet;
use std::ffi::CStr;
use std::fs;
use std::time::Instant;

use super::{Notification, Notifier, NotifyKind};

pub struct IpMonitor {
    last_poll: Option<Instant>,
    last_ip: Option<String>,
    known_ifaces: HashSet<String>,
    poll_secs: u64,
}

impl IpMonitor {
    pub fn new(poll_secs: u64) -> Self {
        Self {
            last_poll: None,
            last_ip: None,
            known_ifaces: list_iface_names().unwrap_or_default(),
            poll_secs,
        }
    }
}

impl super::EventSource for IpMonitor {
    fn poll(&mut self, notifier: &mut Notifier) {
        let current = list_iface_names().unwrap_or_default();
        let new_iface = current.difference(&self.known_ifaces).next().is_some();
        self.known_ifaces = current;

        if !new_iface
            && let Some(last) = self.last_poll
            && last.elapsed().as_secs() < self.poll_secs
        {
            return;
        }
        self.last_poll = Some(Instant::now());

        let entry = match get_active_ipv4() {
            Some(e) => e,
            None => return,
        };

        if !new_iface && self.last_ip.as_deref() == Some(&entry) {
            return;
        }
        self.last_ip = Some(entry.clone());

        notifier.push(Notification::new(NotifyKind::Custom, entry));
    }
}

fn get_active_ipv4() -> Option<String> {
    let mut ifap: *mut libc::ifaddrs = std::ptr::null_mut();
    // SAFETY: getifaddrs 分配链表并写入 ifap，返回值已检查。
    if unsafe { libc::getifaddrs(&mut ifap) } != 0 {
        return None;
    }

    let mut result = None;
    let mut current = ifap;
    while !current.is_null() {
        // SAFETY: current 来自 getifaddrs 链表节点，非空已校验。
        let ifa = unsafe { &*current };
        // SAFETY: ifa_name 是 getifaddrs 返回的合法 C 字符串。
        let name = unsafe { CStr::from_ptr(ifa.ifa_name) }.to_string_lossy();
        if name == "lo" || ifa.ifa_addr.is_null() {
            current = ifa.ifa_next;
            continue;
        }
        // SAFETY: ifa_addr 非空，sa_family 字段始终有效。
        if unsafe { (*ifa.ifa_addr).sa_family } != libc::AF_INET as u16 {
            current = ifa.ifa_next;
            continue;
        }
        if !crate::utils::is_iface_up(&name) {
            current = ifa.ifa_next;
            continue;
        }
        // SAFETY: sa_family == AF_INET 确认可安全转换为 sockaddr_in。
        let addr = unsafe { &*(ifa.ifa_addr as *const libc::sockaddr_in) };
        let o = addr.sin_addr.s_addr.to_ne_bytes();
        if o[0] == 169 && o[1] == 254 {
            current = ifa.ifa_next;
            continue;
        }
        result = Some(format!("{name} {}.{}.{}.{}", o[0], o[1], o[2], o[3]));
        break;
    }
    // SAFETY: ifap 由 getifaddrs 分配，必须释放。
    unsafe {
        libc::freeifaddrs(ifap);
    }
    result
}

fn list_iface_names() -> Option<HashSet<String>> {
    fs::read_dir("/sys/class/net").ok().map(|d| {
        d.flatten()
            .map(|e| e.file_name().to_string_lossy().to_string())
            .filter(|n| n != "lo")
            .collect()
    })
}
