use crate::utils::{self, AppError};

/// 从内核 thermal zone 接口读取 CPU 封装温度。
///
/// 文件内容为一个整数，单位为**千分之一摄氏度**
///（例如 `48123` = 48.123 °C）。
pub struct ThermalMonitor {
    path: String,
}

impl ThermalMonitor {
    pub fn new(path: impl Into<String>) -> Self {
        Self { path: path.into() }
    }

    /// 读取当前 CPU 温度（摄氏度）。
    pub fn poll(&self) -> Result<f32, AppError> {
        let raw = utils::read_trimmed(&self.path)?;
        let millideg: u64 = utils::parse_u64_prefix(&raw).ok_or_else(|| {
            AppError::Parse(format!("thermal: 期望整数，实际为 '{}'", raw))
        })?;
        Ok(millideg as f32 / 1000.0)
    }
}
