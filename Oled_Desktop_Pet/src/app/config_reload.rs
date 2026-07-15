//! 配置热加载 —— 监控 settings.toml 的 mtime，变更时自动重载。

use crate::config::settings as Cfg;

/// 获取文件 mtime（秒），不存在返回 None。
pub fn file_mtime(path: &str) -> Option<i64> {
    let meta = std::fs::metadata(path).ok()?;
    meta.modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .map(|d| d.as_secs() as i64)
}

/// 判断当前是否在夜间静默时段（用于配置热加载和初始判定）。
pub fn is_night(begin: u8, end: u8) -> bool {
    // SAFETY: timespec 在 Linux aarch64 上全零为有效表示。
    let mut ts: libc::timespec = unsafe { std::mem::MaybeUninit::zeroed().assume_init() };
    // SAFETY: clock_gettime 写入栈分配的 ts。
    unsafe { libc::clock_gettime(libc::CLOCK_REALTIME, &mut ts) };
    let mut tm: libc::tm = unsafe { std::mem::MaybeUninit::zeroed().assume_init() };
    if unsafe { libc::localtime_r(&ts.tv_sec, &mut tm) }.is_null() {
        return false;
    }
    let h = tm.tm_hour as u8;
    if begin <= end {
        h >= begin && h < end
    } else {
        h >= begin || h < end
    }
}

/// 获取配置文件的初始 mtime，用于启动时基线。
/// 避免 `cfg_mtime=0` 导致首次检查误判为变更。
pub fn config_mtime() -> i64 {
    let paths = [
        "configs/settings.toml",
        "Oled_Desktop_Pet/configs/settings.toml",
    ];
    paths.iter().find_map(|p| file_mtime(p)).unwrap_or(0)
}

/// 检查配置文件是否更新，若是则重载热更新项。
pub fn reload_if_changed(rt: &mut super::config::RuntimeConfig, cfg_mtime: &mut i64) {
    let paths = [
        "configs/settings.toml",
        "Oled_Desktop_Pet/configs/settings.toml",
    ];
    let found = paths.iter().find(|p| std::path::Path::new(p).exists());
    let path = match found {
        Some(p) => *p,
        None => return,
    };
    let mt = match file_mtime(path) {
        Some(m) => m,
        None => return,
    };
    if mt == *cfg_mtime {
        return;
    }
    *cfg_mtime = mt;

    eprintln!("[配置] 检测到 settings.toml 变更，热加载");
    let raw = match std::fs::read_to_string(path) {
        Ok(r) => r,
        Err(_) => return,
    };
    let table = match raw.parse::<toml::Table>() {
        Ok(t) => t,
        Err(e) => {
            eprintln!("[配置] 解析错误: {e}");
            return;
        }
    };

    rt.alert_temp_high = Cfg::get_f32(&table, "alert.cpu_temp_high", 80.0);
    rt.alert_temp_safe = Cfg::get_f32(&table, "alert.cpu_temp_safe", 75.0);
    rt.alert_mem_high = Cfg::get_f32(&table, "alert.mem_high", 90.0);
    rt.alert_mem_safe = Cfg::get_f32(&table, "alert.mem_safe", 85.0);
    rt.alert_disk_high = Cfg::get_f32(&table, "alert.disk_high", 90.0);
    rt.alert_disk_safe = Cfg::get_f32(&table, "alert.disk_safe", 85.0);
    rt.alert_temp_deadzone = Cfg::get_f32(&table, "alert.temp_deadzone", 2.0);
    rt.alert_throttle_ratio = Cfg::get_f32(&table, "alert.throttle_ratio", 0.7);
    rt.alert_throttle_temp = Cfg::get_f32(&table, "alert.throttle_temp", 75.0);
    rt.night_begin = Cfg::get_u8(&table, "display.night_begin", 23);
    rt.night_end = Cfg::get_u8(&table, "display.night_end", 7);
    rt.night_contrast = Cfg::get_u8(&table, "display.night_contrast", 40);
    rt.malloc_trim_secs = Cfg::get_u64(&table, "poll.malloc_trim_secs", 5);
}
