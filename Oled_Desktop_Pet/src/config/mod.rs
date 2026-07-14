//! 配置管理 —— 从 configs/settings.conf 加载运行时参数。
//!
//! 使用 `settings::load()` 获取键值 Map，
//! 通过 `get_*()` 函数按类型读取，缺省时使用默认值。

pub mod settings;
