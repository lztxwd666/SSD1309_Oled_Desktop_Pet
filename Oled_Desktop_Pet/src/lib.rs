//! OLED 桌宠 —— 系统监控 + OLED 显示。
//!
//! 主循环：4 Hz 渲染 + 1 Hz 监控，self-pipe 信号响应。

pub mod app;
pub mod assets;
pub mod config;
pub mod display;
pub mod model;
pub mod monitor;
pub mod notify;
pub mod renderer;
pub mod resource;
pub mod ui;
pub mod utils;
