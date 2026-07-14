//! 轻量级工具函数和统一错误类型。

pub mod error;

pub use error::AppError;

use std::fs;
use std::io;
use std::path::Path;

/// 读取文件为去空白字符串。
#[inline]
pub(crate) fn read_trimmed(path: impl AsRef<Path>) -> io::Result<String> {
    fs::read_to_string(path.as_ref()).map(|s| s.trim().to_owned())
}

/// 解析字符串前导数字为 u64（遇非数字字符停止）。
#[inline]
pub(crate) fn parse_u64_prefix(s: &str) -> Option<u64> {
    let s = s.trim();
    let digit_end = s
        .as_bytes()
        .iter()
        .position(|b| !b.is_ascii_digit())
        .unwrap_or(s.len());
    if digit_end == 0 {
        return None;
    }
    s[..digit_end].parse().ok()
}

/// 根分区磁盘信息：`(已用字节, 总字节, 使用率%)`。失败返回零值。
pub(crate) fn disk_info() -> (u64, u64, u32) {
    // SAFETY: statvfs 在 Linux aarch64 上全零即有效初始化。
    let mut stat: libc::statvfs = unsafe { std::mem::MaybeUninit::zeroed().assume_init() };
    // SAFETY: statvfs 写入栈分配的 stat，"/" 是合法的 C 字符串路径。
    if unsafe { libc::statvfs(b"/\0".as_ptr() as *const libc::c_char, &mut stat) } != 0 {
        return (0, 0, 0);
    }
    let used = stat.f_blocks.saturating_sub(stat.f_bfree) as u64 * stat.f_frsize as u64;
    let total = stat.f_blocks as u64 * stat.f_frsize as u64;
    let pct = if total > 0 { (used as f64 / total as f64 * 100.0) as u32 } else { 0 };
    (used, total, pct)
}

/// 读取根分区磁盘使用率百分比。返回 None 表示 statvfs 调用失败。
pub(crate) fn disk_usage_pct() -> Option<f32> {
    let (_, _, pct) = disk_info();
    Some(pct as f32)
}

/// 解析字符串为 f32。
#[allow(dead_code)]
#[inline]
pub(crate) fn parse_f32(s: &str) -> Option<f32> {
    s.trim().parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple() {
        assert_eq!(parse_u64_prefix("48123"), Some(48123));
    }

    #[test]
    fn parse_with_suffix() {
        assert_eq!(parse_u64_prefix("48123 kB"), Some(48123));
    }

    #[test]
    fn parse_empty() {
        assert_eq!(parse_u64_prefix(""), None);
    }

    #[test]
    fn parse_non_numeric() {
        assert_eq!(parse_u64_prefix("abc"), None);
    }

    #[test]
    fn parse_zero() {
        assert_eq!(parse_u64_prefix("0"), Some(0));
    }
}
