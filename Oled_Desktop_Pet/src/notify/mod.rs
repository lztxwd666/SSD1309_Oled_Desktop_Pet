//! 通知系统 —— 单槽位 + 即时覆盖 + 2s 自动过期。
//!
//! 事件源（均实现 EventSource trait）：
//! * `ssh` / `usb` / `iface` / `system` / `typec` / `ip`

pub mod iface;
pub mod ip;
pub mod ssh;
pub mod system;
pub mod typec;
pub mod usb;

use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotifyKind {
    SshLogin, UsbInsert, UsbRemove, IfaceUp, IfaceDown,
    TempAlert, MemAlert, DiskAlert, TypeCInsert, TypeCRemove,
    #[allow(dead_code)] Custom,
}

#[derive(Debug, Clone)]
pub struct Notification {
    pub message: String,
}

impl Notification {
    pub fn new(_kind: NotifyKind, message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }
}

pub struct Notifier {
    current: Option<String>,
    since: Option<Instant>,
    duration_secs: u64,
}

impl Notifier {
    pub fn new(duration_secs: u64) -> Self {
        Self { current: None, since: None, duration_secs }
    }

    pub fn push(&mut self, note: Notification) {
        self.current = Some(note.message);
        self.since = Some(Instant::now());
    }

    pub fn current(&mut self) -> Option<&str> {
        if let Some(s) = self.since
            && s.elapsed().as_secs() >= self.duration_secs {
                self.current = None;
                self.since = None;
                return None;
            }
        self.current.as_deref()
    }
}

pub trait EventSource {
    fn poll(&mut self, notifier: &mut Notifier);
}
