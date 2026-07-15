//! 可复用 UI 组件 —— 文本行、状态栏、进度条、分隔线。

use super::layout::{self, Rect};
use crate::display::Framebuffer;
use crate::renderer::{canvas, text};

pub fn status_bar(fb: &mut Framebuffer, area: &Rect, left: &str, right: &str) {
    canvas::fill_rect(fb, area.x, area.y, area.w, area.h);
    text::draw_text_inverted(fb, area.x + 1, area.y, left);
    // 左侧文本过长时跳过右侧时间，避免重叠
    if !right.is_empty() {
        let left_px = text::text_width(left, 1);
        let right_px = text::text_width(right, 1);
        if left_px + right_px + 4 <= area.w.saturating_sub(2) {
            let rx = area.right().saturating_sub(right_px + 1);
            text::draw_text_inverted(fb, rx, area.y, right);
        }
    }
}

pub fn progress_bar(fb: &mut Framebuffer, x: usize, y: usize, w: usize, h: usize, pct: f32) {
    if w < 3 || h < 3 {
        return;
    }
    canvas::draw_rect(fb, x, y, w, h);
    let fill = ((w.saturating_sub(2)) as f32 * pct.clamp(0.0, 100.0) / 100.0) as usize;
    canvas::fill_rect(fb, x + 1, y + 1, fill, h.saturating_sub(2));
}

pub fn draw_dividers(fb: &mut Framebuffer, layout: &layout::Layout) {
    for y in layout.pet.y..layout.pet.bottom() {
        fb.set_pixel(layout.divider_x, y, true);
    }
    let sep_y = layout.status.y.saturating_sub(1);
    canvas::draw_hline_dotted(fb, 0, sep_y, 128);
}
