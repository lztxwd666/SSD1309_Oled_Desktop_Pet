//! 有界格式化 —— 所有指标字符串硬保证 ≤14 字符（70px / 5px）。
//!
//! 布局模式：`[标签 2-3字符] [空格填充] [数值右对齐]` = 14 字符。
//! 测试覆盖全量取值：self / cpu_temp / ram / net_th / rate / mem_compact。

use crate::model::SystemInfo;
use std::time::Duration;

/// 标签-值对齐：`label` 左，`value` 右，空格填充至 14 字符。
fn align(label: &str, value: &str) -> String {
    let total = label.chars().count() + value.chars().count();
    if total >= 14 {
        format!("{}{}", label, value)
    } else {
        format!("{}{}{}", label, " ".repeat(14 - total), value)
    }
}

/// 自身行：RSS 左 + CPU% 右。
/// 格式: `"Me 1584k     0.0%"`
pub fn fmt_self(info: &SystemInfo) -> String {
    let rss = fmt_mem_compact(info.self_rss_kb);
    let cpu = if info.self_cpu_pct >= 100.0 {
        format!("{:3.0}%", info.self_cpu_pct)
    } else {
        format!("{:4.1}%", info.self_cpu_pct)
    };
    align("Me", &format!("{}  {}", rss, cpu))
}

/// CPU 温度 + 频率行：`"CPU    ↑48°C 2.4G"`（箭头紧贴温度值，仿流量箭头，14 字符精确）。
pub fn fmt_cpu_temp_freq(celsius: f32, freq_ghz: f32, trend: i8, throttling: bool) -> String {
    // 箭头紧贴温度值（仿流量 ↑↓ 紧贴速率值），稳定时无箭头
    let arrow = match trend { 1 => "↑", -1 => "↓", _ => "" };
    let warn = if throttling { "!" } else { "" };
    let v = if freq_ghz > 0.0 {
        let f_str = if throttling {
            format!("{:.0}G", freq_ghz.max(0.1))
        } else {
            format!("{:3.1}G", freq_ghz)
        };
        // 温度值去前导空格，箭头紧贴数字
        format!("{}{:.0}°C {}{}", arrow, celsius, warn, f_str)
    } else {
        format!("{}{:.1}°C", arrow, celsius)
    };
    align("CPU", &v)
}

/// CPU 温度行（无频率备份）：`"CPU         48.5°C"`
#[allow(dead_code)]
pub fn fmt_cpu_temp(celsius: f32) -> String {
    fmt_cpu_temp_freq(celsius, 0.0, 0, false)
}

/// RAM 行：`"RAM      1531/3909M"`
pub fn fmt_ram(used_kb: u64, total_kb: u64) -> String {
    let max_kb = used_kb.max(total_kb);
    let (u, t, unit) = if max_kb >= 10_000_000 {
        (
            used_kb as f64 / 1_048_576.0,
            total_kb as f64 / 1_048_576.0,
            "G",
        )
    } else {
        (used_kb as f64 / 1024.0, total_kb as f64 / 1024.0, "M")
    };
    align("RAM", &format!("{:4.0}/{:.0}{}", u, t, unit))
}

/// 网络 + 线程行：线程在左，↑=上传 ↓=下载。
/// 格式: `"Th 1   ↑0B↓0B"` 或 `"Th12   ↑99K↓99K"`
/// ↑↓ 为自定义 5×7 字形（ARROW_UP/DOWN_GLYPH），OLED 上正确渲染。
/// 线程数钳位到 99 保证标签 ≤4 字符。
pub fn fmt_net_th(tx_kibs: f32, rx_kibs: f32, threads: u32) -> String {
    let thr = format!("{}", threads.min(99));
    let net = format!("↑{}↓{}", fmt_rate(tx_kibs).trim(), fmt_rate(rx_kibs).trim());
    align(&format!("Th{}", thr), &net)
}

/// 磁盘行：`"Disk    4.2/29G"`（已用/总量，仿 RAM 格式，14 字符精确）。
pub fn fmt_disk() -> String {
    let (used, total, _) = crate::utils::disk_info();
    let (u, t, unit) = if total >= 1_000_000_000_000 {
        (used as f64 / 1_099_511_627_776.0, total as f64 / 1_099_511_627_776.0, "T")
    } else {
        (used as f64 / 1_073_741_824.0, total as f64 / 1_073_741_824.0, "G")
    };
    align("Disk", &format!("{:.1}/{:.0}{unit}", u, t))
}

/// 当前本地时钟时间：`"14:30"`。
pub fn fmt_clock() -> String {
    // SAFETY: timespec 全零在 Linux aarch64 上是有效表示。
    let mut ts: libc::timespec = unsafe { std::mem::MaybeUninit::zeroed().assume_init() };
    // SAFETY: clock_gettime 写入栈分配的 ts，CLOCK_REALTIME 为标准时钟 ID。
    unsafe { libc::clock_gettime(libc::CLOCK_REALTIME, &mut ts) };
    let secs = ts.tv_sec as libc::time_t;
    // SAFETY: tm 全零初始化，localtime_r 是 glibc 提供的线程安全版本。
    let mut tm: libc::tm = unsafe { std::mem::MaybeUninit::zeroed().assume_init() };
    if unsafe { libc::localtime_r(&secs, &mut tm) }.is_null() {
        return "--:--".to_string();
    }
    format!("{:02}:{:02}", tm.tm_hour, tm.tm_min)
}

/// 程序运行时长（紧凑格式）。
///
/// 输出示例：`"0s"` `"59s"` `"5m30s"` `"2h15m"` `"30d12h"`
/// 最大宽度约 9 字符（`"365d23h"`），≈54px @ 6px/字。
pub fn fmt_uptime(d: Duration) -> String {
    let s = d.as_secs();
    if s < 60 {
        format!("{}s", s)
    } else if s < 3600 {
        format!("{}m{}s", s / 60, s % 60)
    } else if s < 86400 {
        format!("{}h{}m", s / 3600, (s % 3600) / 60)
    } else {
        format!("{}d{}h", s / 86400, (s % 86400) / 3600)
    }
}

/// 内存 → 紧凑表示（≤5 字符）。
fn fmt_mem_compact(kb: u64) -> String {
    if kb >= 100_000_000 {
        format!("{:3.0}G", kb as f64 / 1_048_576.0)
    } else if kb >= 10_000_000 {
        format!("{:3.1}G", kb as f64 / 1_048_576.0)
    } else if kb >= 100_000 {
        format!("{:4.0}M", kb as f64 / 1024.0)
    } else if kb >= 10_000 {
        format!("{:4.1}M", kb as f64 / 1024.0)
    } else {
        format!("{:4}k", kb)
    }
}

/// 网络速率（硬保证 ≤4 字符）。
fn fmt_rate(kibs: f32) -> String {
    if kibs < 0.05 {
        format!("{:>3}B", (kibs * 1024.0) as u32)
    } else if kibs < 10.0 {
        format!("{:3.1}K", kibs)
    } else if kibs < 1000.0 {
        format!("{:3.0}K", kibs)
    } else if kibs < 10000.0 {
        format!("{:3.1}M", kibs / 1024.0)
    } else if kibs < 1000000.0 {
        format!("{:3.0}M", (kibs / 1024.0).min(999.0))
    } else {
        format!("{:3.0}G", (kibs / 1_048_576.0).min(99.0))
    } // ≥1 GiB/s 切到 G
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chars(s: &str) -> usize {
        s.chars().count()
    }

    #[test]
    fn align_pads_to_14() {
        assert_eq!(chars(&align("Me", "1584k  0.0%")), 14);
        assert_eq!(chars(&align("CPU", "48.5°C")), 14);
        assert_eq!(chars(&align("RAM", "1531/3909M")), 14);
    }

    #[test]
    fn self_cases() {
        for rss in [0, 156, 1584, 9999, 10000, 100000, 10_000_000, 999_999_999] {
            for cpu in [0.0, 0.1, 10.0, 99.9, 100.0] {
                let mut info = SystemInfo::default();
                info.self_cpu_pct = cpu;
                info.self_rss_kb = rss;
                let s = fmt_self(&info);
                assert!(
                    chars(&s) <= 14,
                    "fmt_self(cpu={cpu},rss={rss}): '{s}' = {} chars",
                    chars(&s)
                );
            }
        }
    }

    #[test]
    fn cpu_temp_cases() {
        for t in [-10.0, 0.0, 48.5, 99.9, 125.0] {
            let s = fmt_cpu_temp(t);
            assert!(chars(&s) <= 14, "fmt_cpu_temp({t}): '{s}'");
        }
    }

    #[test]
    fn ram_cases() {
        for (u, t) in [
            (0, 3909),
            (975, 3909),
            (3909, 3909),
            (8192, 8192),
            (16384, 16384),
        ] {
            let s = fmt_ram(u, t);
            assert!(
                chars(&s) <= 14,
                "fmt_ram({u},{t}): '{s}' = {} chars",
                chars(&s)
            );
        }
    }

    #[test]
    fn net_th_cases() {
        for tx in [0.0, 0.1, 5.0, 99.9, 999.0, 9999.0] {
            for rx in [0.0, 0.1, 5.0, 99.9] {
                for th in [1, 12, 99] {
                    let s = fmt_net_th(tx, rx, th);
                    assert!(
                        chars(&s) <= 14,
                        "fmt_net_th(tx={tx},rx={rx},th={th}): '{s}' = {} chars",
                        chars(&s)
                    );
                }
            }
        }
    }

    #[test]
    fn rate_max_4() {
        for kibs in [
            0.0, 0.01, 0.1, 9.9, 10.0, 99.9, 100.0, 999.0, 1000.0, 9999.0,
        ] {
            let s = fmt_rate(kibs);
            assert!(chars(&s) <= 4, "fmt_rate({kibs}): '{s}'");
        }
    }

    #[test]
    fn mem_compact_max_5() {
        for kb in [
            0, 156, 1584, 9999, 10000, 100000, 999999, 10_000_000, 99_999_999,
        ] {
            let s = fmt_mem_compact(kb);
            assert!(chars(&s) <= 5, "fmt_mem_compact({kb}): '{s}'");
        }
    }
}
