//! 开机动画流程 —— 逐步初始化并显示进度。

use crate::display;
use crate::monitor;
use crate::resource;
use crate::utils::AppError;

use super::render::draw_centered_screen;
use super::signal;

/// 执行完整开机序列：OLED → 字体 → 监控 → Ready。
/// 任意步骤收到信号则返回 Err。
pub fn run_boot_sequence(
    display: &mut display::Display, pipe_read: i32,
    boot_ms: i32, done_ms: i32, font_path: &str,
    thermal_path: &str, iface_priority: &[String],
) -> Result<monitor::SystemMonitor, AppError> {
    // 辅助：单步执行，信号中断返回 Err
    let step = |display: &mut display::Display, progress: u8, label: &str| -> Result<(), AppError> {
        draw_centered_screen(&mut display.framebuffer, "Booting...", progress, label);
        display.render()?;
        let timeout = if progress == 100 { done_ms } else { boot_ms };
        if signal::poll_signal(pipe_read, timeout)? {
            return Err(AppError::Config("开机被信号中断".into()));
        }
        Ok(())
    };

    step(display, 20, "Init OLED")?;

    resource::font_loader::load_font(Some(std::path::Path::new(font_path)))
        .map(|d| { crate::renderer::font::set_font_data(d); })
        .unwrap_or_else(|e| eprintln!("[警告] 字体加载失败: {e}"));
    step(display, 50, "Load font")?;

    let monitor = monitor::SystemMonitor::from_paths(
        thermal_path.to_string(), iface_priority.to_vec())?;
    step(display, 80, "Init monitor")?;

    step(display, 100, "Ready")?;

    display.framebuffer.clear();
    display.render()?;
    println!("[初始化] 监控 + OLED 就绪\n");
    Ok(monitor)
}
