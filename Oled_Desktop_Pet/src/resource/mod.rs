//! 资源加载 —— 从磁盘读取素材并缓存。
//!
//! 当前提供：
//! * `font_loader` — 从 `configs/font.txt` 加载自定义字体，回退到内嵌默认

pub mod font_loader;
