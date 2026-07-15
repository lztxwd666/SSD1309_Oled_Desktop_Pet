//! 屏幕渲染协调 —— 将各 UI 组件组装为完整画面。
//!
//! 渲染顺序至关重要：精灵 → 分隔线 → 指标 → 状态栏。
//! 精灵缓冲区中的 Off 像素会清除已绘制内容，因此必须在分隔线和指标之前绘制。

use std::time::Duration;

use crate::display::Framebuffer;
use crate::model::SystemInfo;
use crate::notify::Notifier;
use crate::renderer::{canvas, text};
use crate::ui::{fmt, layout::LAYOUT, widget};

/// 进度条参数（开机 / 关机共用）。
const BAR_W: usize = 80;
const BAR_H: usize = 6;
const BAR_X: usize = 20;
const BAR_Y: usize = 34;
const BAR_PCT_GAP: usize = 3;

/// 状态栏右侧切换间隔（帧数）：uptime ↔ 时钟。
const STATUS_TOGGLE: u64 = 4;

// ── 开机/关机动画 ─────────────────────────────────────────

pub fn draw_centered_screen(fb: &mut Framebuffer, title: &str, progress: u8, label: &str) {
    fb.clear();

    let title_w = title.len() * 6;
    let title_x = (128usize.saturating_sub(title_w)) / 2;
    text::draw_text(fb, title_x, 18, title);

    canvas::draw_rect(fb, BAR_X, BAR_Y, BAR_W, BAR_H);
    let fill = ((BAR_W.saturating_sub(2)) as u32 * progress.min(100) as u32 / 100) as usize;
    if fill > 0 {
        canvas::fill_rect(fb, BAR_X + 1, BAR_Y + 1, fill, BAR_H.saturating_sub(2));
    }
    let pct_str = format!("{}%", progress);
    let pct_x = BAR_X + BAR_W + BAR_PCT_GAP;
    text::draw_text(fb, pct_x, BAR_Y, &pct_str);

    let label_w = label.len() * 6;
    let label_x = (128usize.saturating_sub(label_w)) / 2;
    text::draw_text(fb, label_x, 44, label);
}

// ── 主屏幕渲染 ────────────────────────────────────────────

pub fn render_screen(
    fb: &mut Framebuffer,
    info: &SystemInfo,
    uptime: Duration,
    notifier: &mut Notifier,
    cycle: u64,
    blink_interval: u64,
) {
    render_pet(fb, cycle, blink_interval);
    widget::draw_dividers(fb, &LAYOUT);
    render_metrics(fb, info);
    render_status(fb, uptime, notifier, cycle);
}

// ── 精灵 + 眨眼 + 每核条 ──────────────────────────────────

fn render_pet(fb: &mut Framebuffer, cycle: u64, blink_interval: u64) {
    use crate::assets;
    use embedded_graphics::image::{ImageDrawable, ImageRaw};
    use embedded_graphics::pixelcolor::BinaryColor;

    let raw: ImageRaw<BinaryColor> = ImageRaw::new(&assets::BASE_SPRITE, 64);
    raw.draw(fb).ok();

    // 眨眼间隔由配置控制。
    if cycle.is_multiple_of(blink_interval) {
        let m = 3usize;
        // 统一尺寸避免两眼渲染偏差
        let ew = assets::EYE_BOX_W + m * 2;
        let eh = assets::EYE_BOX_H + m + 2;
        let slit_y = (assets::EYE_L_Y + eh).saturating_sub(eh / 3 + 1).min(63);

        for (ex, ey) in [
            (assets::EYE_L_X, assets::EYE_L_Y),
            (assets::EYE_R_X, assets::EYE_R_Y),
        ] {
            let x0 = ex.saturating_sub(m);
            let y0 = ey.saturating_sub(m);
            let bot = (ey + eh).min(64);
            for py in y0..bot {
                for px in x0..(ex + ew).min(128) {
                    fb.set_pixel(px, py, true);
                }
            }
            for row in 0..2usize {
                let sy = (slit_y + row).min(63);
                for px in (x0 + 1)..(x0 + ew).saturating_sub(1).min(128) {
                    fb.set_pixel(px, sy, false);
                }
            }
        }
    }
}

/// 宠物区底部绘制每核利用率微型条（1px 高 × 4 核）。
fn render_percore_bars(fb: &mut Framebuffer, info: &SystemInfo) {
    let bar_y = LAYOUT.pet.bottom().saturating_sub(4);
    let bar_w = 52; // 总宽度
    let bar_x = 2;

    for i in 0..4 {
        let y = bar_y + i;
        if y >= LAYOUT.pet.bottom() {
            break;
        }
        let fill = (bar_w as f32 * info.per_core_pct[i].clamp(0.0, 100.0) / 100.0) as usize;
        if fill > 0 {
            canvas::fill_rect(fb, bar_x, y, fill, 1);
        }
    }
}

// ── 指标面板 ──────────────────────────────────────────────

fn render_metrics(fb: &mut Framebuffer, info: &SystemInfo) {
    use crate::renderer::text::draw_text_packed;
    use crate::ui::layout;

    let x = LAYOUT.metrics.x;
    let w = LAYOUT.metrics.w;
    let gap = layout::GROUP_GAP;
    let mut y = LAYOUT.metrics.y;

    // 自身进程
    text::draw_text_packed(fb, x, y, &fmt::fmt_self(info));
    y += layout::ROW_H + gap;

    // CPU 温度 + 频率 + 趋势箭头 + 降频警告
    text::draw_text_packed(
        fb,
        x,
        y,
        &fmt::fmt_cpu_temp_freq(
            info.cpu_temp_celsius,
            info.cpu_freq_ghz,
            info.temp_trend,
            info.cpu_throttling,
        ),
    );
    y += layout::ROW_H;

    // CPU 利用率进度条
    let pct_str = format!("{:4.1}%", info.cpu_usage_pct);
    let pct_px = pct_str.len() * 5;
    let bar_w = w.saturating_sub(pct_px + 1);
    widget::progress_bar(fb, x, y + 1, bar_w, 5, info.cpu_usage_pct);
    draw_text_packed(fb, x + bar_w + 1, y, &pct_str);
    y += layout::ROW_H + gap;

    // 内存 + 磁盘（存储组）
    text::draw_text_packed(fb, x, y, &fmt::fmt_ram(info.mem_used_kb, info.mem_total_kb));
    y += layout::ROW_H;
    text::draw_text_packed(fb, x, y, &fmt::fmt_disk());
    y += layout::ROW_H + gap;

    // 网络 + 线程（底部）
    text::draw_text_packed(
        fb,
        x,
        y,
        &fmt::fmt_net_th(
            info.net_tx_rate_kibs,
            info.net_rx_rate_kibs,
            info.self_threads,
        ),
    );

    // 宠物区底部每核条
    render_percore_bars(fb, info);
}

// ── 状态栏 ────────────────────────────────────────────────

fn render_status(fb: &mut Framebuffer, uptime: Duration, notifier: &mut Notifier, cycle: u64) {
    // 状态栏使用监控帧计数（cycle/4），保持 1 Hz 切换速度
    let mc = cycle / 4;
    let right = if mc % (STATUS_TOGGLE * 2) < STATUS_TOGGLE {
        fmt::fmt_uptime(uptime)
    } else {
        fmt::fmt_clock()
    };
    widget::status_bar(fb, &LAYOUT.status, notifier.current().unwrap_or(""), &right);
}

// ── 终端辅助 ──────────────────────────────────────────────

pub fn fmt_rate_term(kibs: f32) -> String {
    if kibs < 0.1 {
        format!("{:>4}B/s", (kibs * 1024.0) as u32)
    } else if kibs < 99.9 {
        format!("{:>4.1}K/s", kibs)
    } else if kibs < 999.0 {
        format!("{:>4.0}K/s", kibs)
    } else {
        format!("{:>4.1}M/s", kibs / 1024.0)
    }
}
