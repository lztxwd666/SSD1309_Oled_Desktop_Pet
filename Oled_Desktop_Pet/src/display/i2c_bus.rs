//! Linux I2C 总线底层封装。
//!
//! 直接操作 `/dev/i2c-N` 设备节点，通过 ioctl 绑定从机地址后
//! 使用 write 系统调用发送数据。

use std::fs;
use std::io::{self, Write};
use std::os::unix::io::AsRawFd;

const I2C_SLAVE: u64 = 0x0703;

pub struct I2cBus {
    file: fs::File,
    #[allow(dead_code)]
    addr: u8,
}

impl I2cBus {
    pub fn open(bus: u8, addr: u8) -> io::Result<Self> {
        let path = format!("/dev/i2c-{}", bus);
        let file = fs::OpenOptions::new().read(true).write(true).open(&path)?;
        // SAFETY: file 是刚打开的合法 fd，I2C_SLAVE ioctl 仅设置从机地址，无副作用。
        let ret = unsafe { libc::ioctl(file.as_raw_fd(), I2C_SLAVE, addr as u32) };
        if ret < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(Self { file, addr })
    }

    /// 发送 I2C 命令（控制字节 0x00 前缀）。使用栈缓冲区避免堆分配。
    pub fn write_command(&mut self, bytes: &[u8]) -> io::Result<()> {
        const MAX: usize = 255;
        if bytes.len() > MAX {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("I2C 命令过长: {} > {}", bytes.len(), MAX),
            ));
        }
        let mut buf = [0u8; 256];
        buf[0] = 0x00;
        buf[1..1 + bytes.len()].copy_from_slice(bytes);
        self.file.write_all(&buf[..1 + bytes.len()])
    }

    /// 发送 GDDRAM 数据（控制字节 0x40 前缀）。使用栈缓冲区避免堆分配。
    pub fn write_data(&mut self, bytes: &[u8]) -> io::Result<()> {
        const MAX: usize = 255;
        if bytes.len() > MAX {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("I2C 数据过长: {} > {}", bytes.len(), MAX),
            ));
        }
        let mut buf = [0u8; 256];
        buf[0] = 0x40;
        buf[1..1 + bytes.len()].copy_from_slice(bytes);
        self.file.write_all(&buf[..1 + bytes.len()])
    }
}
