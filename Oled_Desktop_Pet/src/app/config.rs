//! 运行时配置 —— 从 configs/settings.toml 一次性加载全部参数。
//!
//! 文件不存在时全部使用默认值，程序正常运行。

use crate::config;

/// 运行时配置。
pub struct RuntimeConfig {
    // ── 显示 ──
    pub i2c_bus: u8,
    pub i2c_addr: u8,
    pub night_begin: u8,
    pub night_end: u8,
    pub night_contrast: u8,
    pub blink_interval: u64,
    // ── 轮询 ──
    pub poll_interval_ms: i32,
    pub boot_frame_ms: i32,
    pub done_frame_ms: i32,
    pub shutdown_frame_ms: u64,
    pub malloc_trim_secs: u64,
    // ── 通知 ──
    pub notify_duration_secs: u64,
    pub ssh_poll_secs: u64,
    pub ssh_renotify_secs: u64,
    pub ip_poll_secs: u64,
    // ── 告警 ──
    pub alert_temp_high: f32,
    pub alert_temp_safe: f32,
    pub alert_mem_high: f32,
    pub alert_mem_safe: f32,
    pub alert_disk_high: f32,
    pub alert_disk_safe: f32,
    pub alert_temp_deadzone: f32,
    pub alert_throttle_ratio: f32,
    pub alert_throttle_temp: f32,
    // ── 路径 ──
    pub thermal_path: String,
    pub font_config_path: String,
    // ── 网络 ──
    pub net_iface_priority: Vec<String>,
}

pub fn load() -> RuntimeConfig {
    let raw = config::settings::load();
    use config::settings as S;

    fn clamp_i32(v: u64) -> i32 { v.min(i32::MAX as u64) as i32 }

    RuntimeConfig {
        i2c_bus: S::get_u8(&raw, "display.i2c_bus", 1),
        i2c_addr: u8::from_str_radix(
            S::get_str(&raw, "display.i2c_addr", "0x3C").trim_start_matches("0x"), 16,
        ).unwrap_or_else(|e| { eprintln!("[配置] display.i2c_addr 无效: {e}"); 0x3C }),
        night_begin: S::get_u8(&raw, "display.night_begin", 23),
        night_end: S::get_u8(&raw, "display.night_end", 7),
        night_contrast: S::get_u8(&raw, "display.night_contrast", 40),
        blink_interval: S::get_u64(&raw, "display.blink_interval", 6),
        // 轮询
        poll_interval_ms: clamp_i32(S::get_u64(&raw, "poll.interval_ms", 250)),
        boot_frame_ms: clamp_i32(S::get_u64(&raw, "boot.frame_ms", 250)),
        done_frame_ms: clamp_i32(S::get_u64(&raw, "boot.done_ms", 400)),
        shutdown_frame_ms: clamp_i32(S::get_u64(&raw, "shutdown.frame_ms", 200)) as u64,
        malloc_trim_secs: S::get_u64(&raw, "poll.malloc_trim_secs", 5),
        // 通知
        notify_duration_secs: S::get_u64(&raw, "notify.duration_secs", 2),
        ssh_poll_secs: S::get_u64(&raw, "notify.ssh_poll_secs", 5),
        ssh_renotify_secs: S::get_u64(&raw, "notify.ssh_renotify_secs", 300),
        ip_poll_secs: S::get_u64(&raw, "notify.ip_poll_secs", 2),
        // 告警
        alert_temp_high: S::get_f32(&raw, "alert.cpu_temp_high", 80.0),
        alert_temp_safe: S::get_f32(&raw, "alert.cpu_temp_safe", 75.0),
        alert_mem_high: S::get_f32(&raw, "alert.mem_high", 90.0),
        alert_mem_safe: S::get_f32(&raw, "alert.mem_safe", 85.0),
        alert_disk_high: S::get_f32(&raw, "alert.disk_high", 90.0),
        alert_disk_safe: S::get_f32(&raw, "alert.disk_safe", 85.0),
        alert_temp_deadzone: S::get_f32(&raw, "alert.temp_deadzone", 2.0),
        alert_throttle_ratio: S::get_f32(&raw, "alert.throttle_ratio", 0.7),
        alert_throttle_temp: S::get_f32(&raw, "alert.throttle_temp", 75.0),
        // 路径
        thermal_path: S::get_str(&raw, "path.thermal", "/sys/class/thermal/thermal_zone0/temp"),
        font_config_path: S::get_str(&raw, "path.font_config", "configs/font.txt"),
        // 网络
        net_iface_priority: S::get_str_list(&raw, "net.iface_priority", &["eth0", "end0", "wlan0"]),
    }
}
