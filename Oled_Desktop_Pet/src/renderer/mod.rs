//! 软件渲染层 —— 在 1-bit 帧缓冲区上绘制文本和几何图形。
//!
//! * `font` — 5×7 + 4×6 位图字体
//! * `text` — 文本渲染（正常、紧凑、反色模式）
//! * `canvas` — 几何图形（矩形、填充、点线）

pub mod canvas;
pub mod font;
pub mod text;
