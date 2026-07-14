//! 配置系统 —— 从 configs/settings.toml 读取 TOML 格式配置。
//!
//! 文件不存在时全部使用默认值，程序正常运行。
//! 使用 toml crate 解析，支持标准 TOML 类型和嵌套表。

use std::fs;

/// 配置文件搜索路径（按优先级）。
const CONFIG_PATHS: &[&str] = &[
    "configs/settings.toml",
    "Oled_Desktop_Pet/configs/settings.toml",
];

/// 加载 TOML 配置，文件不存在时返回空 Table。
pub fn load() -> toml::Table {
    for path in CONFIG_PATHS {
        match fs::read_to_string(path) {
            Ok(content) => {
                return content.parse::<toml::Table>().unwrap_or_else(|e| {
                    eprintln!("[配置] {path} 解析错误: {e}，使用默认值");
                    toml::Table::new()
                });
            }
            Err(_) => continue,
        }
    }
    eprintln!("[配置] 未找到配置文件，使用默认值");
    toml::Table::new()
}

// ── 统一值解析 ────────────────────────────────────────────

/// 解析 `section.key` 或顶级 `key` → `Option<&Value>`。
fn get_value<'a>(root: &'a toml::Table, key: &str) -> Option<&'a toml::Value> {
    if let Some((section, k)) = key.split_once('.') {
        root.get(section)?.as_table()?.get(k)
    } else {
        root.get(key)
    }
}

// ── 类型化访问器 ──────────────────────────────────────────

pub fn get_str(root: &toml::Table, key: &str, default: &str) -> String {
    get_value(root, key)
        .and_then(|v| v.as_str())
        .unwrap_or(default)
        .to_string()
}

pub fn get_u8(root: &toml::Table, key: &str, default: u8) -> u8 {
    get_value(root, key)
        .and_then(|v| v.as_integer())
        // 防止负值或超范围值静默截断：i2c_bus=256 如不钳位会变成 0
        .and_then(|i| u8::try_from(i).ok())
        .unwrap_or(default)
}

#[allow(dead_code)]
pub fn get_u32(root: &toml::Table, key: &str, default: u32) -> u32 {
    get_value(root, key)
        .and_then(|v| v.as_integer())
        .and_then(|i| u32::try_from(i).ok())
        .unwrap_or(default)
}

pub fn get_u64(root: &toml::Table, key: &str, default: u64) -> u64 {
    get_value(root, key)
        .and_then(|v| v.as_integer())
        .and_then(|i| u64::try_from(i).ok())
        .unwrap_or(default)
}

pub fn get_f32(root: &toml::Table, key: &str, default: f32) -> f32 {
    get_value(root, key)
        .and_then(|v| v.as_float())
        .map(|f| f as f32)
        .unwrap_or(default)
}

#[allow(dead_code)]
pub fn get_usize(root: &toml::Table, key: &str, default: usize) -> usize {
    get_value(root, key)
        .and_then(|v| v.as_integer())
        .and_then(|i| usize::try_from(i).ok())
        .unwrap_or(default)
}

pub fn get_str_list(root: &toml::Table, key: &str, default: &[&str]) -> Vec<String> {
    match get_value(root, key).and_then(|v| v.as_array()) {
        Some(arr) => arr
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect(),
        None => default.iter().map(|s| s.to_string()).collect(),
    }
}
