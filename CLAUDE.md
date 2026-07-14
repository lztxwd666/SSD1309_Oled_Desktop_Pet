# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

OLED desktop pet for Raspberry Pi CM5 — a 128×64 monochrome SSD1309 display driven via I2C, showing a static ferris crab sprite alongside real-time system metrics and event notifications.

**Current state**: ~3500 lines Rust, 46 tests, three external dependencies (`libc`, `embedded-graphics`, `toml`). Monitor (thermal + cpu + memory + network + process + per-core + cpufreq), display driver, renderer (font + text + canvas), UI layout engine with bounded formatting, notification system (SSH/USB/iface/system/typec/IP alerts — single-slot 2s), TOML config, compile-time sprite generation (PNG → 1-bit via build.rs with automatic eye detection), and ferris blink animation are all implemented and stable.

## Workspace

- Root `Cargo.toml`: workspace, `members = ["Oled_Desktop_Pet"]`, `resolver = "2"`
- `Oled_Desktop_Pet/`: application crate (edition 2024), dependencies: `libc = "0.2"`, `embedded-graphics = "0.8"`, `toml = "0.8"`
- Build dependency: `image = "0.24"` (PNG decoding only)
- `Cargo.lock` at workspace root

## Commands

```bash
cd /home/pi/I2C_Display_Driver/Oled_Desktop_Pet
cargo build                           # build (dev)
cargo build --release                 # build (release, ~1.1 MB binary)
cargo run                             # run (OLED + terminal, 1 Hz)
cargo test                            # all 46 tests
cargo test <name>                     # single test, e.g. cargo test fmt_self
cargo check                           # fast check, no codegen
cargo clippy                          # lint (if installed)
sudo i2cdetect -y 1                   # scan I2C bus (should show 0x3C)
```

Platform: aarch64, Debian 12 Bookworm, kernel 6.12, Rust 1.96.1.

## Hardware (fixed)

- **Display**: SSD1309 128×64 monochrome OLED, I2C addr `0x3C`
- **Bus**: I2C-1 (`/dev/i2c-1`) on GPIO10 (SDA) / GPIO11 (SCL)
- Pi user is in `i2c` group — no `sudo` needed
- `/boot/firmware/config.txt`:
  ```
  dtparam=i2c_arm=on
  dtoverlay=i2c1-pi5,pins_10_11
  ```

## Architecture

```
main.rs  ── 装配层（初始化→开机动画→主循环→关机动画）
  ├── app/           应用程序协调（从 main.rs 拆分）
  │   ├── config.rs       RuntimeConfig 加载（TOML 一次性解析）
  │   ├── signal.rs       Self-pipe 信号处理 + CLOCK_BOOTTIME
  │   ├── render.rs       屏幕渲染协调（精灵→分隔线→指标→状态栏）
  │   └── shutdown.rs     关机动画 + OLED 休眠
  ├── model/          数据类型：SystemInfo（16 字段）
  ├── monitor/        系统监控：thermal, cpu, cpufreq, memory, network, percore, process
  ├── notify/         通知系统：ssh, usb, iface, system, typec, ip（单槽位 2s 即时覆盖）
  ├── ui/             布局引擎：layout（坐标常量），widget（组件），fmt（有界格式化，14 字符保证）
  ├── renderer/       软件渲染：font（5×7+4×6+°↑↓），text（正常/反色/紧凑），canvas（几何）
  ├── display/        硬件驱动：i2c_bus → ssd1309 → framebuffer（1024 字节）
  ├── utils/          工具：AppError（统一错误），read_trimmed，parse_u64_prefix
  ├── config/         配置：settings（TOML 解析 + 类型化访问器）
  └── resource/       资源：font_loader（configs/font.txt → 自定义字体）
```

## Key data flow

```
SystemMonitor::poll_all() → SystemInfo
    → [event_sources.poll(&mut notifier)]   ← SSH/USB/iface/typec/IP 事件源
    → [system_alerts.check(&info, ...)]      ← CPU 温度/内存/磁盘告警
    → ui::fmt::fmt_*(&info)                  ← bounded strings (≤14 chars)
    → ui::widget::*  → renderer::text::*
    → render_pet()  + blink_eyes()           ← compile-time sprite + RGBA 眼球检测
    → display::Framebuffer
    → Display::render()                      ← SSD1309 page-by-page (8×128 bytes)
```

## SystemInfo fields (model/system_info.rs)

`cpu_temp_celsius`, `cpu_usage_pct`, `mem_total_kb`, `mem_used_kb`, `mem_usage_pct`, `net_rx_bytes`, `net_tx_bytes`, `net_rx_rate_kibs`, `net_tx_rate_kibs`, `self_cpu_pct`, `self_rss_kb`, `self_vm_kb`, `self_threads`, `cpu_freq_ghz`, `per_core_pct: [f32; 4]`, `timestamp`

## Critical implementation details

### Self-pipe signal handling (app/signal.rs)

Pattern: `libc::pipe()` → signal handler writes 1 byte via `write()` → main loop uses `libc::poll()` with timeout. Replaces `thread::sleep` for instant (< 1ms) Ctrl+C response.

- `sigaction()` with `SA_RESTART | SA_SIGINFO` — not `signal()` (deterministic behavior)
- `PIPE_WRITE_FD`: `AtomicI32` static, stored with `SeqCst`, loaded in handler with `Acquire`
- `MaybeUninit::zeroed().assume_init()` for FFI structs (`sigaction`, `timespec`, `statvfs`)
- `CLOCK_BOOTTIME` via `libc::clock_gettime` — monotonic clock that includes suspend time, optimal for uptime
- Configuration values that become `i32` (e.g. `poll_interval_ms`) are clamped via `clamp_i32()` to prevent negative timeout → infinite poll block

### SSD1309 driver (display/)

- I2C: `write(&[0x00, cmd..])` = command, `write(&[0x40, data..])` = GDDRAM
- **`0xAD 0x8A`**: SSD1309 DC-DC converter — mandatory, missing causes garbled pixels
- **`0x20 0x02`**: page addressing mode (not horizontal `0x00`)
- Frame push: 8 pages × 128 bytes (avoids Pi 5 RP1 I2C bulk-transfer limits)
- 100 ms delay after charge-pump + DC-DC enable
- I2C writes use stack-allocated `[u8; 256]` buffer (zero heap allocation)
- `Display::sleep()` → `0xAE` command at shutdown

### Bounded formatting (ui/fmt.rs)

All format functions have parametric tests covering full value ranges. Every string guaranteed ≤14 chars (70 px ÷ 5 px/char). Tests use `.chars().count()` (↑↓° are multi-byte UTF-8).

Layout: pet=56px, divider=56, metrics=70px (14 chars), status=128×7 (inverted). 6 rows fit in 56px via `GROUP_GAP=2`, `ROW_H=8`.

### Render order (app/render.rs)

**Order is critical**: `render_pet → draw_dividers → render_metrics → render_status`. The 64×64 sprite buffer has Off pixels in columns 56–63 that would clear previously drawn content if drawn after dividers/metrics.

The `status_bar` widget must only use `draw_text_inverted` and `fill_rect` — **never raw `fb.set_pixel`** in the status bar area. Raw framebuffer operations break the SSD1309 page-based rendering pipeline.

### Notification system (notify/)

- **Single-slot model**: `Notifier` holds one notification. New `push()` overwrites immediately. Expires after 2 seconds.
- `current()` returns `Option<&str>`, expires inline (no separate `tick()` needed).
- Event source poll order matters: `IfaceMonitor` must be LAST so `up`/`down` overwrites `IpMonitor`'s IP display.
- `IfaceMonitor`: combines multiple simultaneous changes into one notification (`"eth0 eth1 up"`)
- `UsbMonitor`: retries product name read via `pending_names` if sysfs `product` file is not yet available
- `IpMonitor`: uses `getifaddrs` + operstate check, skips link-local (169.254.x.x), 2s polling interval

### Self-monitoring (monitor/process.rs)

Reads `/proc/self/stat`: `Δ(utime+stime) / ticks_per_sec / Δwall_clock × 100` — matches `top -p <pid>` exactly. Uses `sysconf(_SC_CLK_TCK)` and `sysconf(_SC_PAGESIZE)`.

### Network monitoring (monitor/network.rs)

Dynamic interface selection on every poll:
1. Default route interface (from `/proc/net/route`, Destination=00000000, Mask=00000000)
2. Fallback: most-active up interface (from `/proc/net/dev`)
3. Last resort: first existing interface from priority list

All field parsing uses iterators (no `Vec::collect()` in hot path).

### Per-core CPU (monitor/percore.rs)

Reads `/proc/stat` lines for `cpu0`..`cpu3`. Displays as 4 × 1px horizontal bars at bottom of pet area (y=52-55).

### Font system (renderer/font.rs)

- 5×7 `FALLBACK_FONT_DATA` (475 bytes, 95 glyphs)
- Custom glyphs: `°` (3×3 circle), `↑` (upload arrow), `↓` (download arrow)
- 4×6 `SMALL_FONT_DATA` (380 bytes) — designed but NOT enabled
- `set_font_data([u8;475])` for runtime override from `resource::font_loader`
- Config file: `configs/font.txt` (hex format, one line per char code)

### Compile-time sprite + eye detection (build.rs + assets.rs)

- `build.rs`: reads `assets/images/ferris.png` → alpha composite onto black → BT.601 luminance → threshold binarization (80) → `[u8; 512]` (64×64 horizontal-byte layout, MSB left)
- **Eye detection**: scans original RGBA pixels for bright spots (R,G,B > 180) in facial region → outputs `EYE_L_X/Y`, `EYE_R_X/Y` constants for blink animation
- Output: `OUT_DIR/sprites.rs` → `include!()` in `assets.rs` → `ImageRaw<BinaryColor>` at runtime
- Blink animates at `cycle % 2 == 0` (once per 2s): draws crab-colored eyelid (ON) + thin black slit (OFF) over detected eye positions

## Configuration (configs/settings.toml)

8 sections, all optional with defaults: `[display]`, `[poll]`, `[boot]`, `[shutdown]`, `[alert]`, `[path]`, `[net]`, `[sprite]`. Dual-path search (`configs/settings.toml` + `Oled_Desktop_Pet/configs/settings.toml`) supports both `cargo run` from crate dir and `cargo run -p` from workspace root.

## Deployment

```bash
./scripts/install-service.sh    # compile + install systemd service (auto-detects I2C bus)
sudo ./scripts/uninstall-service.sh  # stop + remove service + binary
```

- systemd service: `ProtectSystem=strict`, `NoNewPrivileges=yes`, `PrivateTmp=yes`
- `Restart=on-failure`, `RestartSec=5`
- Binary: `/usr/local/bin/oled-pet`, config: `/etc/oled-pet/`

## Annotation standards

- All code comments in **Chinese** (中文)
- Avoid `=` or `-` as comment separators
- Technical terms (CPU, RAM, I2C, SSD1309, RSS) remain in English
- `unsafe` blocks annotated with `// SAFETY:` comments

## Sub-agents

- `embedded-reviewer`: Audits Rust embedded code with emphasis on security and performance. Invoke via Agent tool with `subagent_type: "embedded-reviewer"`.
