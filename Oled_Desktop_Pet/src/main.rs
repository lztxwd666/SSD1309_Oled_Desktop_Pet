//! OLED 桌宠 —— 系统监控 + OLED 显示。
//!
//! 主循环：4 Hz 渲染 + 1 Hz 监控，self-pipe 信号响应。

mod app;
mod assets;
mod config;
mod display;
mod model;
mod monitor;
mod notify;
mod renderer;
mod resource;
mod ui;
mod utils;

use std::io::{self, Write};
use std::sync::atomic::Ordering;

use app::signal;
use notify::{
    EventSource, Notifier, iface::IfaceMonitor, ip::IpMonitor, ssh::SshMonitor,
    system::SystemAlerts, typec::TypeCMonitor, usb::UsbMonitor,
};
use utils::AppError;

fn main() {
    if let Err(e) = run() {
        eprintln!("\n[致命错误] {e}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), AppError> {
    // ── 初始化 ──
    let (pipe_read, pipe_write) = signal::create_pipe()?;
    signal::PIPE_WRITE_FD.store(pipe_write, Ordering::SeqCst);
    signal::install_signal_handlers();

    let mut rt = app::config::load();
    let started = signal::boottime_now();

    let mut display = display::Display::open(rt.i2c_bus, rt.i2c_addr)?;

    // 开机动画（信号中断则提前关机）
    let mut monitor = match app::boot::run_boot_sequence(
        &mut display,
        pipe_read,
        rt.boot_frame_ms,
        rt.done_frame_ms,
        &rt.font_config_path,
        &rt.thermal_path,
        &rt.net_iface_priority,
    ) {
        Ok(m) => m,
        Err(_) => {
            return app::shutdown::shutdown(
                &mut display,
                false,
                rt.shutdown_frame_ms,
                rt.done_frame_ms as u64,
                pipe_read,
            );
        }
    };

    // ── 通知系统 ──
    let alert_cfg = notify::system::AlertConfig {
        temp_high: rt.alert_temp_high,
        temp_safe: rt.alert_temp_safe,
        mem_high: rt.alert_mem_high,
        mem_safe: rt.alert_mem_safe,
        disk_high: rt.alert_disk_high,
        disk_safe: rt.alert_disk_safe,
    };
    let mut notifier = Notifier::new(rt.notify_duration_secs);
    let mut event_sources: Vec<Box<dyn EventSource>> = vec![
        Box::new(SshMonitor::new(rt.ssh_poll_secs, rt.ssh_renotify_secs)),
        Box::new(UsbMonitor::new()),
        Box::new(TypeCMonitor::new()),
        Box::new(IpMonitor::new(rt.ip_poll_secs)),
        Box::new(IfaceMonitor::new()),
    ];
    let mut system_alerts = SystemAlerts::new(alert_cfg);

    // ── 主循环 ──
    let mut cycle: u64 = 1;
    let mut info = monitor.poll_all();
    let mut last_temp = info.cpu_temp_celsius;
    let mut night = false;
    let mut night_check: u64 = 0;
    let mut cfg_mtime: i64 = 0;

    loop {
        // 1 Hz 监控帧
        if cycle.is_multiple_of(4) {
            info = monitor.poll_all();
            info.temp_trend = trend(info.cpu_temp_celsius, last_temp, rt.alert_temp_deadzone);
            last_temp = info.cpu_temp_celsius;
            info.cpu_throttling = monitor.last_cpu_max > 0.0
                && monitor.last_cpu_freq < monitor.last_cpu_max * rt.alert_throttle_ratio
                && info.cpu_temp_celsius > rt.alert_throttle_temp;

            for src in &mut event_sources {
                src.poll(&mut notifier);
            }
            system_alerts.check(&info, &mut notifier);

            // 终端
            print!(
                "\rMe {:>4.1}% {:>5}k  CPU {:>4.1}°C {:>5.1}%  RAM {:>4.1}%  ↑{:>5} ↓{:>5}",
                info.self_cpu_pct,
                info.self_rss_kb,
                info.cpu_temp_celsius,
                info.cpu_usage_pct,
                info.mem_usage_pct,
                app::render::fmt_rate_term(info.net_tx_rate_kibs),
                app::render::fmt_rate_term(info.net_rx_rate_kibs)
            );
            let _ = io::stdout().flush();

            // 夜间模式 —— 每 60 秒重新评估（避免每秒 clock_gettime + localtime_r）
            night_check += 1;
            if night_check >= 60 {
                night_check = 0;
                let now_night = app::config_reload::is_night(rt.night_begin, rt.night_end);
                if now_night != night {
                    night = now_night;
                    display.set_contrast(if night { rt.night_contrast } else { 0xCF })?;
                }
            }

            // 配置热加载
            app::config_reload::reload_if_changed(&mut rt, &mut cfg_mtime);
        }

        // 渲染
        let uptime = signal::boottime_now().saturating_sub(started);
        display.framebuffer.clear();
        app::render::render_screen(
            &mut display.framebuffer,
            &info,
            uptime,
            &mut notifier,
            cycle,
            rt.blink_interval,
        );
        display.render()?;

        if signal::poll_signal(pipe_read, rt.poll_interval_ms)? {
            break;
        }

        cycle += 1;
        if cycle.is_multiple_of(rt.malloc_trim_secs * 4) {
            // SAFETY: malloc_trim 是 glibc 扩展，参数 0 表示释放所有可释放内存，
            // 无副作用，返回值忽略。在非 glibc 平台上不可用，本项目目标为 Debian/glibc。
            let _ = unsafe { libc::malloc_trim(0) };
        }
    }

    app::shutdown::shutdown(
        &mut display,
        true,
        rt.shutdown_frame_ms,
        rt.done_frame_ms as u64,
        pipe_read,
    )
}

fn trend(curr: f32, prev: f32, deadzone: f32) -> i8 {
    let d = curr - prev;
    if d > deadzone {
        1
    } else if d < -deadzone {
        -1
    } else {
        0
    }
}
