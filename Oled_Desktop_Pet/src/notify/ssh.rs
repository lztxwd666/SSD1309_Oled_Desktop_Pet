//! SSH 登录事件源 —— 通过 journalctl 监控 sshd 会话事件。
//!
//! 去重策略：同一用户 5 分钟内不重复通知，超时后重新通知。
//! 避免短时间重连刷屏，同时保证新会话不被永久忽略。

use std::collections::HashMap;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use super::{Notification, Notifier, NotifyKind};

/// journalctl 命令超时秒数，防止 systemd-journald 卡死时阻塞主循环。
const JOURNALCTL_TIMEOUT_SECS: u64 = 5;

/// SSH 登录事件源 —— 定期检查 journalctl。
pub struct SshMonitor {
    seen: HashMap<String, Instant>,
    last_poll: Option<Instant>,
    poll_secs: u64,
    renotify_secs: u64,
}

impl SshMonitor {
    pub fn new(poll_secs: u64, renotify_secs: u64) -> Self {
        Self {
            seen: HashMap::new(),
            last_poll: None,
            poll_secs,
            renotify_secs,
        }
    }
}

impl super::EventSource for SshMonitor {
    fn poll(&mut self, notifier: &mut Notifier) {
        if let Some(last) = self.last_poll
            && last.elapsed().as_secs() < self.poll_secs
        {
            return;
        }
        self.last_poll = Some(Instant::now());

        let output = match run_journalctl() {
            Some(o) => o,
            None => return,
        };

        let stdout = String::from_utf8_lossy(&output);
        for line in stdout.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || !trimmed.contains("session opened for user") {
                continue;
            }

            let user = extract_between(trimmed, "for user ", "(").unwrap_or("?");
            let now = Instant::now();
            // 5 分钟内同一用户不重复通知；超时或首次登录均触发
            if let Some(last) = self.seen.get(user) {
                if now.duration_since(*last).as_secs() < self.renotify_secs {
                    continue;
                }
            }
            self.seen.insert(user.to_string(), now);

            notifier.push(Notification::new(
                NotifyKind::SshLogin,
                format!("SSH {user}"),
            ));
        }
    }
}

/// 带超时保护的 journalctl 调用。
/// journald 卡死时不会阻塞主线程超过 `JOURNALCTL_TIMEOUT_SECS` 秒。
fn run_journalctl() -> Option<Vec<u8>> {
    let child = Command::new("journalctl")
        .args(["-u", "ssh", "--no-pager", "-o", "cat", "-n", "3"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;

    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let result = child.wait_with_output();
        let _ = tx.send(result);
    });

    match rx.recv_timeout(Duration::from_secs(JOURNALCTL_TIMEOUT_SECS)) {
        Ok(Ok(out)) if out.status.success() => Some(out.stdout),
        _ => None,
    }
}

fn extract_between<'a>(s: &'a str, prefix: &str, suffix: &str) -> Option<&'a str> {
    let start = s.find(prefix)? + prefix.len();
    let rest = &s[start..];
    let end = rest.find(suffix).unwrap_or(rest.len());
    Some(rest[..end].trim())
}
