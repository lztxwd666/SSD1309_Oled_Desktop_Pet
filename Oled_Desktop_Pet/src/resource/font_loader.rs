//! 字体加载器 —— 从配置文件加载自定义字体，嵌入默认回退。
//!
//! 字体文件格式（纯文本）：
//! ```text
//! # 注释行以 # 开头
//! # 格式: <ASCII码> <5个十六进制字节>
//! 32 00 00 00 00 00
//! 33 00 00 5F 00 00
//! ```
//!
//! 文件存放路径：`configs/font.txt`。
//! 如果文件不存在或格式错误，自动回退到内嵌默认字体。

use std::fs;
use std::path::Path;

use crate::utils::AppError;

/// 字体配置文件的默认搜索路径。
pub const FONT_CONFIG_PATH: &str = "configs/font.txt";

/// 加载字体数据。
///
/// 返回 `[u8; 475]`（95 个可打印 ASCII 字符 × 5 列）。
/// 优先从 `configs/font.txt` 加载，失败则使用内嵌默认字体。
pub fn load_font(path: Option<&Path>) -> Result<[u8; 475], AppError> {
    let path = path.unwrap_or_else(|| Path::new(FONT_CONFIG_PATH));

    match fs::read_to_string(path) {
        Ok(content) => parse_font_file(&content),
        Err(_) => {
            // 文件不存在或不可读 → 使用默认字体（静默降级）
            Ok(default_font_data())
        }
    }
}

/// 解析字体文件内容。
fn parse_font_file(content: &str) -> Result<[u8; 475], AppError> {
    let mut data = [0u8; 475];
    let mut parsed = [false; 95]; // 已解析的字符标记

    for (line_no, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        // 跳过空行和注释
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // 格式: <char_code> <5 hex bytes>
        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        if parts.len() < 6 {
            return Err(AppError::Config(format!(
                "字体文件第 {} 行格式错误: 需要 <字符码> <5字节十六进制>, 实际: {}",
                line_no + 1,
                trimmed
            )));
        }

        let char_code: usize = parts[0].parse().map_err(|_| {
            AppError::Config(format!("字体文件第 {} 行字符码无效: {}", line_no + 1, parts[0]))
        })?;

        if !(32..=126).contains(&char_code) {
            return Err(AppError::Config(format!(
                "字体文件第 {} 行字符码超出范围 (32-126): {}",
                line_no + 1,
                char_code
            )));
        }

        let idx = char_code - 32;
        let base = idx * 5;

        for col in 0..5 {
            let hex_str = parts[1 + col];
            // 去掉可选的 "0x" 前缀
            let hex_str = hex_str.strip_prefix("0x").unwrap_or(hex_str);
            data[base + col] = u8::from_str_radix(hex_str, 16).map_err(|_| {
                AppError::Config(format!(
                    "字体文件第 {} 行第 {} 列: 无效十六进制 '{}'",
                    line_no + 1,
                    col + 1,
                    parts[1 + col]
                ))
            })?;
        }

        parsed[idx] = true;
    }

    // 未定义的字符用默认字体填充
    let defaults = default_font_data();
    for (i, is_parsed) in parsed.iter().enumerate() {
        if !is_parsed {
            let base = i * 5;
            data[base..base + 5].copy_from_slice(&defaults[base..base + 5]);
        }
    }

    Ok(data)
}

/// 内嵌默认 5×7 字体（与旧版 `font.rs` 中的 FONT_DATA 完全一致）。
fn default_font_data() -> [u8; 475] {
    *super::super::renderer::font::FALLBACK_FONT_DATA
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_without_file_returns_default() {
        let result = load_font(Some(Path::new("/nonexistent/font.txt")));
        assert!(result.is_ok());
        let data = result.unwrap();
        // 空格（索引 0）应为全零
        assert_eq!(&data[0..5], &[0, 0, 0, 0, 0]);
    }

    #[test]
    fn parse_valid_font_line() {
        let content = "65 7E 11 11 11 7E\n"; // 'A'
        let data = parse_font_file(content).unwrap();
        let idx_a = (65 - 32) * 5;
        assert_eq!(&data[idx_a..idx_a + 5], &[0x7E, 0x11, 0x11, 0x11, 0x7E]);
    }

    #[test]
    fn parse_missing_chars_filled_with_default() {
        let content = "# empty file, no characters defined\n";
        let data = parse_font_file(content).unwrap();
        // 所有字符应由默认字体填充
        assert_eq!(&data[0..5], &[0, 0, 0, 0, 0]); // 空格
    }
}
