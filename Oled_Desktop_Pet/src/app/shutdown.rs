//! 关机流程 —— 动画、清理、OLED 休眠。

use std::sync::atomic::Ordering;

use crate::display;
use crate::utils::AppError;

use super::render::draw_centered_screen;
use super::signal;

/// 执行关机动画并退出。
///
/// `show_animation` 控制是否显示完整关机动画：
/// * 主循环中退出 → 显示动画
/// * 开机过程中退出 → 直接清屏（避免开机/关机动画冲突）
///
/// 使用 `poll_signal` 替代 `thread::sleep`，关机动画期间仍可响应
/// 第二次 Ctrl+C 以跳过动画直接退出。
pub fn shutdown(
    display: &mut display::Display,
    show_animation: bool,
    sfd: u64,
    dfd: u64,
    pipe_read: i32,
) -> Result<(), AppError> {
    // 辅助：立即清屏退出
    fn quick_exit(display: &mut display::Display) -> Result<(), AppError> {
        display.framebuffer.clear();
        display.render()?;
        display.sleep()?;
        close_pipe_write_fd();
        Ok(())
    }

    if show_animation {
        println!("\n[退出] 收到终止信号，正在关机…");
        draw_centered_screen(&mut display.framebuffer, "Shutting down...", 50, "Saving");
        display.render()?;
        if signal::poll_signal(pipe_read, sfd as i32)? {
            return quick_exit(display);
        }
        draw_centered_screen(&mut display.framebuffer, "Shutting down...", 100, "Goodbye");
        display.render()?;
        if signal::poll_signal(pipe_read, dfd as i32)? {
            return quick_exit(display);
        }
    }

    // 清空画面并关闭显示（0xAE = 休眠模式）
    display.framebuffer.clear();
    display.render()?;
    display.sleep()?;

    close_pipe_write_fd();

    Ok(())
}

/// 关闭 self-pipe 写端 fd，进程退出前最后清理。
fn close_pipe_write_fd() {
    let fd = signal::PIPE_WRITE_FD.load(Ordering::Relaxed);
    if fd >= 0 {
        // SAFETY: close() 对合法 fd 是幂等的，进程退出前最后清理。
        unsafe {
            libc::close(fd);
        }
    }
}
