//! 绘图原语 —— 在帧缓冲区上绘制几何图形与填充。
//!
//! 当前提供矩形边框、填充矩形、水平线及其点线变体。

use crate::display::Framebuffer;

/// 绘制矩形边框（空心）。越界自动裁剪与 fill_rect 保持一致。
pub fn draw_rect(fb: &mut Framebuffer, x: usize, y: usize, w: usize, h: usize) {
    if w == 0 || h == 0 || x >= 128 || y >= 64 {
        return;
    }
    let x2 = x.saturating_add(w.saturating_sub(1)).min(127);
    let y2 = y.saturating_add(h.saturating_sub(1)).min(63);

    for px in x..=x2 {
        fb.set_pixel(px, y, true);
        fb.set_pixel(px, y2, true);
    }
    for py in y..=y2 {
        fb.set_pixel(x, py, true);
        fb.set_pixel(x2, py, true);
    }
}

/// 绘制实心矩形（填充）。
pub fn fill_rect(fb: &mut Framebuffer, x: usize, y: usize, w: usize, h: usize) {
    if w == 0 || h == 0 || x >= 128 || y >= 64 {
        return;
    }
    let x2 = (x + w).min(128);
    let y2 = (y + h).min(64);
    for py in y..y2 {
        for px in x..x2 {
            fb.set_pixel(px, py, true);
        }
    }
}

/// 绘制水平实线。
#[allow(dead_code)]
pub fn draw_hline(fb: &mut Framebuffer, x: usize, y: usize, len: usize) {
    for i in 0..len {
        fb.set_pixel(x.saturating_add(i), y, true);
    }
}

/// 绘制水平点线（每隔一个像素）。
pub fn draw_hline_dotted(fb: &mut Framebuffer, x: usize, y: usize, len: usize) {
    for i in (0..len).step_by(2) {
        fb.set_pixel(x.saturating_add(i), y, true);
    }
}

/// 绘制垂直点线。
#[allow(dead_code)]
pub fn draw_vline_dotted(fb: &mut Framebuffer, x: usize, y: usize, len: usize) {
    for i in (0..len).step_by(2) {
        fb.set_pixel(x, y.saturating_add(i), true);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rect_does_not_panic_at_bounds() {
        let mut fb = Framebuffer::new();
        draw_rect(&mut fb, 0, 0, 128, 64);
        draw_rect(&mut fb, 200, 200, 50, 50);
    }

    #[test]
    fn fill_rect_works() {
        let mut fb = Framebuffer::new();
        fill_rect(&mut fb, 0, 0, 10, 10);
        assert!(fb.get_pixel(5, 5));
        assert!(!fb.get_pixel(15, 5));
    }

    #[test]
    fn dotted_line_spacing() {
        let mut fb = Framebuffer::new();
        draw_hline_dotted(&mut fb, 0, 0, 10);
        assert!(fb.get_pixel(0, 0));
        assert!(!fb.get_pixel(1, 0));
        assert!(fb.get_pixel(2, 0));
    }

    #[test]
    fn empty_rect_noop() {
        let mut fb = Framebuffer::new();
        fb.set_pixel(10, 10, true);
        draw_rect(&mut fb, 10, 10, 0, 0);
        draw_rect(&mut fb, 10, 10, 1, 0);
        assert!(fb.get_pixel(10, 10));
    }
}
