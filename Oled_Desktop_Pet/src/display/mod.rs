//! 显示硬件驱动层 —— SSD1309 OLED 通过 I2C 通信。
//!
//! Display 结构体将 I2C 总线、SSD1309 控制器和帧缓冲区组合为统一接口。

mod framebuffer;
mod i2c_bus;
mod ssd1309;

pub use framebuffer::Framebuffer;

use crate::utils::AppError;
use i2c_bus::I2cBus;
use ssd1309::Ssd1309;

/// OLED 显示器顶层句柄。
pub struct Display {
    driver: Ssd1309,
    pub framebuffer: Framebuffer,
}

impl Display {
    /// 打开并初始化显示器。
    pub fn open(bus_id: u8, addr: u8) -> Result<Self, AppError> {
        let bus = I2cBus::open(bus_id, addr)?;
        let driver = Ssd1309::init(bus)?;
        Ok(Self { driver, framebuffer: Framebuffer::new() })
    }

    /// 将帧缓冲区内容推送到 OLED。
    pub fn render(&mut self) -> Result<(), AppError> {
        Ok(self.driver.push_frame(&self.framebuffer)?)
    }

    /// 设置对比度（0-255）。
    pub fn set_contrast(&mut self, val: u8) -> Result<(), AppError> {
        Ok(self.driver.set_contrast(val)?)
    }

    /// 关闭 OLED 显示（进入休眠模式，0xAE）。
    pub fn sleep(&mut self) -> Result<(), AppError> {
        Ok(self.driver.sleep()?)
    }
}
