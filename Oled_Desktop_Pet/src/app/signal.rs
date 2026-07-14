//! Self-pipe 信号处理基础设施。
//!
//! 设计：信号处理器向管道写 1 字节，主线程 poll 管道读端。
//! 零 CPU 轮询、瞬时响应、异步信号安全（仅 write 系统调用）。

use std::io;
use std::sync::atomic::{AtomicI32, Ordering};
use std::time::Duration;

/// self-pipe 写端 fd。
///
/// 初始值 -1（管道未创建），初始化后设为管道写端 fd。
/// 信号处理器从此读取 fd 并向管道写入 1 字节以唤醒主线程 poll。
pub static PIPE_WRITE_FD: AtomicI32 = AtomicI32::new(-1);

// ── 管道创建 ──────────────────────────────────────────────

/// 创建匿名管道，返回 (读端 fd, 写端 fd)。
pub fn create_pipe() -> io::Result<(i32, i32)> {
    let mut fds = [0i32; 2];
    // SAFETY: pipe() 填栈分配的 [i32;2]，POSIX 保证有效，返回值已检查。
    if unsafe { libc::pipe(fds.as_mut_ptr()) } < 0 {
        return Err(io::Error::last_os_error());
    }
    // 防止子进程意外继承管道 fd
    // SAFETY: fds[0]/fds[1] 是刚创建的合法 fd，fcntl(F_SETFD) 无副作用。
    unsafe {
        libc::fcntl(fds[0], libc::F_SETFD, libc::FD_CLOEXEC);
        libc::fcntl(fds[1], libc::F_SETFD, libc::FD_CLOEXEC);
    }
    Ok((fds[0], fds[1]))
}

// ── 信号处理器 ────────────────────────────────────────────

/// 使用 sigaction 注册 SIGINT / SIGTERM 处理器（行为确定，优于 signal()）。
///
/// SA_RESTART 确保 poll 等系统调用在信号处理器返回后自动重启，
/// 从而在处理器写入管道后立即检测到管道可读。
pub fn install_signal_handlers() {
    // SAFETY: sigaction 在 Linux aarch64 上所有字段归零即有效初始化。
    let mut sa: libc::sigaction = unsafe { std::mem::MaybeUninit::zeroed().assume_init() };
    sa.sa_sigaction = handle_shutdown_sa as *const () as usize;
    sa.sa_flags = libc::SA_RESTART | libc::SA_SIGINFO;
    // SAFETY: sa 指向有效栈变量，oldact 为 null（不关心旧处理器）。
    unsafe {
        libc::sigaction(libc::SIGINT, &sa, std::ptr::null_mut());
        libc::sigaction(libc::SIGTERM, &sa, std::ptr::null_mut());
    }
}

extern "C" fn handle_shutdown_sa(_sig: i32, _info: *mut libc::siginfo_t, _ctx: *mut libc::c_void) {
    // Acquire 与主线程的 SeqCst store 配对，形式上正确且 aarch64 上零成本。
    let fd = PIPE_WRITE_FD.load(Ordering::Acquire);
    if fd >= 0 {
        // SAFETY: write 是异步信号安全的唯一输出系统调用之一。
        let _ = unsafe { libc::write(fd, &1u8 as *const u8 as *const libc::c_void, 1) };
    }
}

// ── Poll 封装 ─────────────────────────────────────────────

/// 阻塞等待 pipe_read 可读，最长 `timeout_ms` 毫秒。
///
/// 返回 `Ok(true)` = 收到信号（管道可读），`Ok(false)` = 超时。
/// poll 被信号中断（EINTR）时自动重试 —— 此时信号处理器已写入管道，
/// 重试的 poll 会立即返回可读。
pub fn poll_signal(pipe_read: i32, timeout_ms: i32) -> io::Result<bool> {
    let mut pfd = libc::pollfd {
        fd: pipe_read,
        events: libc::POLLIN,
        revents: 0,
    };

    loop {
        // SAFETY: pfd 指向栈分配 pollfd，nfds=1，timeout 为 i32 毫秒值。
        let ret = unsafe { libc::poll(&mut pfd, 1, timeout_ms) };
        if ret > 0 {
            // 管道可读 → 排空缓冲（单字节，但排空多个以防极端情况）
            let mut buf = [0u8; 8];
            // SAFETY: pipe_read 是合法 fd，buf 是栈数组，read 排空管道数据。
            unsafe {
                libc::read(pipe_read, buf.as_mut_ptr() as *mut libc::c_void, 8);
            }
            return Ok(true);
        } else if ret == 0 {
            return Ok(false); // 超时，无信号
        } else {
            // ret < 0
            let err = io::Error::last_os_error();
            if err.raw_os_error() == Some(libc::EINTR) {
                // 信号中断了 poll → retry（此时管道已有数据）
                continue;
            }
            return Err(err);
        }
    }
}

// ── 时钟 ──────────────────────────────────────────────────

/// 读取 CLOCK_BOOTTIME（挂起期间持续计时 + 单调不跳变 —— uptime 最优时钟源）。
pub fn boottime_now() -> Duration {
    // SAFETY: timespec 在 Linux aarch64 上全零即有效。
    let mut ts: libc::timespec = unsafe { std::mem::MaybeUninit::zeroed().assume_init() };
    // SAFETY: clock_gettime 写入栈分配的 ts 指针，无副作用。
    unsafe { libc::clock_gettime(libc::CLOCK_BOOTTIME, &mut ts) };
    Duration::new(ts.tv_sec as u64, ts.tv_nsec as u32)
}
