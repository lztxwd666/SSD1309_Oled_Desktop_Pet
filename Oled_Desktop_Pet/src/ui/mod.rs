//! UI 布局引擎 —— 将 OLED 屏幕组合为多个区域。
//!
//! 子模块：
//! * `layout` — 布局常量与区域定义
//! * `widget` — 可复用 UI 组件
//! * `fmt` — 有界格式化（所有指标字符串硬保证不越界）

pub mod fmt;
pub mod layout;
pub mod widget;
