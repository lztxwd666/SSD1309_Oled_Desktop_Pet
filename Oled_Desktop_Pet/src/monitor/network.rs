use std::fs;
use std::io::{BufRead, BufReader};
use std::time::Instant;

use crate::utils::AppError;

/// 内核网络接口统计文件。
const NET_DEV_PATH: &str = "/proc/net/dev";
/// 内核路由表。
const NET_ROUTE_PATH: &str = "/proc/net/route";

/// 网络吞吐率监控器。
///
/// 每次 poll 重新评估最优监控接口：
/// 1. 有默认路由（上网）→ 监控承载默认路由的接口
/// 2. 无默认路由（仅本地）→ 监控上行/下行总流量最大的已启用接口
/// 3. 兜底：任一 up 接口 → 任一存在接口
pub struct NetworkMonitor {
    iface: String,
    prev_rx: u64,
    prev_tx: u64,
    prev_time: Instant,
    rx_rate: f32,
    tx_rate: f32,
    priority: Vec<String>,
    poll_count: u64,
}

impl NetworkMonitor {
    #[allow(dead_code)]
    pub fn new() -> Result<Self, AppError> {
        Self::new_with_priority(vec!["eth0".into(), "end0".into(), "wlan0".into()])
    }

    pub fn new_with_priority(priority: Vec<String>) -> Result<Self, AppError> {
        let iface = select_best_iface(&priority)?;
        let (rx, tx) = read_iface_stats(&iface)?;
        Ok(Self {
            iface,
            prev_rx: rx,
            prev_tx: tx,
            prev_time: Instant::now(),
            rx_rate: 0.0,
            tx_rate: 0.0,
            priority,
            poll_count: 0,
        })
    }

    #[inline]
    pub fn total_rx(&self) -> u64 {
        self.prev_rx
    }

    #[inline]
    pub fn total_tx(&self) -> u64 {
        self.prev_tx
    }

    #[inline]
    pub fn rx_rate_kbps(&self) -> f32 {
        self.rx_rate
    }

    #[inline]
    pub fn tx_rate_kbps(&self) -> f32 {
        self.tx_rate
    }

    #[inline]
    #[allow(dead_code)]
    pub fn iface_name(&self) -> &str {
        &self.iface
    }

    /// 轮询：每 10 帧重新评估最优接口，其余帧直接读取当前接口统计。
    pub fn poll(&mut self) -> Result<(), AppError> {
        self.poll_count += 1;
        // 每 10 帧重新评估最优接口（接口变化在 1 Hz 下 10 秒足够检测）
        if self.poll_count.is_multiple_of(10) {
            if let Ok(best) = select_best_iface(&self.priority)
                && best != self.iface
            {
                let (rx, tx) = read_iface_stats(&best)?;
                self.iface = best;
                self.prev_rx = rx;
                self.prev_tx = tx;
                self.prev_time = Instant::now();
                self.rx_rate = 0.0;
                self.tx_rate = 0.0;
                return Ok(());
            }
        }

        let (rx, tx) = read_iface_stats(&self.iface)?;
        let now = Instant::now();
        let elapsed = now.duration_since(self.prev_time).as_secs_f32();

        self.rx_rate = if elapsed > 0.0 {
            (rx.saturating_sub(self.prev_rx) as f32 / 1024.0) / elapsed
        } else {
            0.0
        };
        self.tx_rate = if elapsed > 0.0 {
            (tx.saturating_sub(self.prev_tx) as f32 / 1024.0) / elapsed
        } else {
            0.0
        };

        self.prev_rx = rx;
        self.prev_tx = tx;
        self.prev_time = now;
        Ok(())
    }
}

// ── 接口选择 ──────────────────────────────────────────────

/// 选择最优监控接口（三级降级）：
///
/// 1. 默认路由接口（上网流量）
/// 2. 总流量最大的已启用接口（本地 SSH 等）
/// 3. 兜底：任一存在的接口
fn select_best_iface(priority: &[String]) -> Result<String, AppError> {
    if let Some(iface) = default_route_iface()
        && is_up(&iface)
    {
        return Ok(iface);
    }
    if let Some(iface) = most_active_up_iface() {
        return Ok(iface);
    }
    for candidate in priority {
        if fs::metadata(format!("/sys/class/net/{}", candidate)).is_ok() {
            return Ok(candidate.to_string());
        }
    }

    Err(AppError::NotFound("未找到任何非 loopback 网络接口".into()))
}

/// 从 /proc/net/route 获取默认路由（Destination=00000000, Mask=00000000）的接口名。
/// 字段布局（Linux ≥2.6.14）：Iface(0) Dest(1) Gateway(2) Flags(3) RefCnt(4)
///                              Use(5) Metric(6) Mask(7) MTU(8) Window(9) IRTT(10)
fn default_route_iface() -> Option<String> {
    let f = fs::File::open(NET_ROUTE_PATH).ok()?;
    for line in BufReader::new(f).lines().skip(1) {
        let line = line.ok()?;
        let mut fields = line.split_whitespace();
        let iface = fields.next()?.to_string();
        // Dest=字段1, Mask=字段7（从 0 起为索引 1 和 7）
        if fields.next() == Some("00000000") && fields.nth(5) == Some("00000000") {
            return Some(iface);
        }
    }
    None
}

/// 从 /proc/net/dev 找到 total(rx+tx) 最大的已启用非 lo 接口。
/// 字段偏移（Linux ≥2.6.14）：rx_bytes=索引0, tx_bytes=索引8（跳过7个字段）
fn most_active_up_iface() -> Option<String> {
    let f = fs::File::open(NET_DEV_PATH).ok()?;
    let mut best_name: Option<String> = None;
    let mut best_total: u64 = 0;

    for line in BufReader::new(f).lines().skip(2) {
        let line = line.ok()?;
        let trimmed = line.trim_start();
        if let Some(colon_pos) = trimmed.find(':') {
            let name = trimmed[..colon_pos].trim().to_string();
            if name == "lo" || !is_up(&name) {
                continue;
            }
            // 使用迭代器替代 Vec::collect()，避免堆分配
            let mut rest = trimmed[colon_pos + 1..].split_whitespace();
            let rx: u64 = rest.next()?.parse().ok()?;
            // 跳过 7 个字段到 tx（索引 8）
            let tx: u64 = rest.nth(7)?.parse().ok()?;
            let total = rx.saturating_add(tx);
            if total > best_total {
                best_total = total;
                best_name = Some(name);
            }
        }
    }
    best_name
}

/// 检查接口是否启用。
fn is_up(name: &str) -> bool {
    fs::read_to_string(format!("/sys/class/net/{}/operstate", name))
        .map(|s| s.trim() == "up")
        .unwrap_or(false)
}

// ── 原始数据读取 ──────────────────────────────────────────

/// 从 /proc/net/dev 解析接口的 (rx_bytes, tx_bytes)。
fn read_iface_stats(iface: &str) -> Result<(u64, u64), AppError> {
    let f = fs::File::open(NET_DEV_PATH)?;
    let needle = format!("{}:", iface);

    for line in BufReader::new(f).lines() {
        let line = line?;
        let trimmed = line.trim_start();
        if !trimmed.starts_with(&needle) {
            continue;
        }
        // 使用迭代器替代 Vec::collect()：字段 0=接口名, 1=rx, ..., 9=tx
        let mut fields = trimmed.split_whitespace();
        let _name = fields.next(); // 跳过接口名
        let rx: u64 = fields
            .next()
            .ok_or_else(|| AppError::Parse("net/dev rx 字段缺失".into()))?
            .parse()
            .map_err(|_| AppError::Parse("net/dev rx".into()))?;
        // 跳过 7 个字段到 tx
        let tx: u64 = fields
            .nth(7)
            .ok_or_else(|| AppError::Parse("net/dev tx 字段缺失".into()))?
            .parse()
            .map_err(|_| AppError::Parse("net/dev tx".into()))?;
        return Ok((rx, tx));
    }

    Err(AppError::NotFound(format!(
        "接口 '{}' 在 {} 中未找到",
        iface, NET_DEV_PATH
    )))
}
