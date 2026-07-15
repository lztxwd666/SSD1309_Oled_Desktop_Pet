# OLED Desktop Pet

Raspberry Pi CM5 的 128×64 单色 OLED（SSD1309）系统监控桌面宠物，通过 I2C 驱动

显示Rust螃蟹宠物（带眨眼动画）+ 实时系统指标 + 事件通知

<p align="center">
  <img src="example.png" alt="OLED Desktop Pet 运行截图" width="512">
</p>

## 功能

- **系统监控**：CPU 温度/频率（趋势箭头+降频警告）、CPU 利用率+每核微型条、内存、网络 ↑↓ 速率、磁盘 已用/总量、自身进程 RSS/CPU%
- **通知系统**：SSH 登录、USB 插拔、网口 up/down、Type-C、IP 地址、CPU/内存/磁盘告警
- **动画**：4 Hz 渲染，ferris 眨眼，RGBA 眼球自动检测
- **夜间模式**：可配置时段自动降低 OLED 对比度
- **配置热加载**：告警阈值等参数修改后自动生效，无需重启
- **安全**：systemd 沙箱、self-pipe 信号处理、全 unsafe 审计

## 硬件

- SSD1309 128×64 OLED，I2C 地址 `0x3C`，总线 I2C-1
- GPIO10 (SDA) / GPIO11 (SCL)
- `/boot/firmware/config.txt` 需启用 I2C

## 快速开始

```bash
cd Oled_Desktop_Pet
cargo build --release
cargo run
```

安装为 systemd 服务（开机自启）：

```bash
./scripts/install-service.sh
```

## 配置

编辑 `configs/settings.toml`（25 个参数，带注释）

标注 `[热加载]` 的项修改后自动生效，其余需重启

## 命令

```bash
cargo build              # 开发构建
cargo build --release    # 发布构建（~1.1 MB 二进制）
cargo run                # 运行
cargo test               # 46 个测试
cargo test <name>        # 单个测试
cargo clippy             # 静态检查
sudo i2cdetect -y 1      # 扫描 I2C 总线
```

## 架构

```
lib.rs (11 行) ── 模块声明
main.rs (165 行) ── 装配层（初始化→开机动画→主循环→关机动画）
├── app/          boot / config / config_reload / render / shutdown / signal
├── monitor/      cpu / cpufreq / memory / network / percore / process / thermal
├── notify/       iface / ip / ssh / system / typec / usb
├── renderer/     canvas / font / text
├── display/      framebuffer / i2c_bus / ssd1309
├── ui/           fmt / layout / widget
├── model/        SystemInfo
├── config/       TOML 设置
├── resource/     字体加载
└── utils/        错误类型 + 工具函数
```

## 部署

```bash
./scripts/install-service.sh     # 编译+安装 systemd 服务
sudo ./scripts/uninstall-service.sh  # 停止+移除
```

systemd 服务配置：`ProtectSystem=strict`，`NoNewPrivileges=yes`，自动探测 I2C 总线

## 依赖

仅三个外部 crate：`libc`、`embedded-graphics`、`toml`

构建时依赖 `image`（PNG 解码）

## 平台支持

### 主要支持（开发与测试目标）

| 项目 | 要求 |
|------|------|
| **硬件** | Raspberry Pi（CM4/CM5/4B/5B），aarch64 |
| **系统** | Debian 12 Bookworm + systemd |
| **内核** | Linux 6.6+（使用 procfs/sysfs 标准接口） |
| **libc** | glibc 2.36+ |
| **Rust** | 1.96+（edition 2024） |

在此配置下，所有功能保证正常工作：系统监控、通知系统、夜间模式、配置热加载、systemd 服务部署

### 其他发行版兼容性

以下发行版**可能**可运行但未测试，部分功能受限：

| 发行版 | 已知限制 |
|--------|----------|
| **Ubuntu（aarch64）** | 需确认 I2C 总线号,可能需要 `apt install i2c-tools` |
| **Fedora（aarch64）** | SSH 通知需将 `settings.toml` 中 `journalctl -u ssh` 改为 `-u sshd`（Fedora 的 SSH 服务单元名为 `sshd`） |
| **Arch Linux ARM** | 正常需安装 `i2c-tools`，确认 `/boot/config.txt` 中 I2C 已启用 |
| **Alpine Linux** | **无法编译**使用 musl libc，项目依赖 `malloc_trim` 等 glibc 扩展 |
| **Devuan / Gentoo（openrc）** | SSH 登录通知不可用,依赖 `journalctl`（systemd 专属），其他功能正常 |
| **非 systemd 发行版** | systemd 服务部署（`install-service.sh`）不可用，需手动编写 init 脚本,`journalctl` 相关功能（SSH 通知）失效 |

### 功能对内核接口的依赖

| 功能 | 依赖路径 | 缺失时行为 |
|------|----------|------------|
| CPU 温度 | `/sys/class/thermal/thermal_zone0/temp` | 温度显示为 0°C，可通过 `settings.toml` → `path.thermal` 配置其他 zone |
| CPU 频率 | `/sys/devices/system/cpu/cpu0/cpufreq/scaling_cur_freq` | 频率显示为 0，降频警告失效（容器/无 cpufreq 驱动时常见） |
| 网络流量 | `/proc/net/dev` + `/proc/net/route` | 网络指标归零，接口选择使用配置文件中的优先级列表 |
| SSH 登录通知 | `journalctl -u ssh` | 非 systemd 系统上 SSH 通知静默失效 |
| 磁盘用量 | `statvfs("/")` | 失败时磁盘行显示 0 |
| Type-C 通知 | `/sys/class/typec/` | 无 Type-C 控制器时静默跳过 |
| 每核利用率 | `/proc/stat` 中 `cpu0`..`cpu3` 行 | 仅显示前 4 核（OLED 空间限制） |

### 架构

仅支持 **aarch64**（64 位 ARM，小端序）

项目中多处 `MaybeUninit::zeroed().assume_init()` 的 SAFETY 前提基于 aarch64 Linux 的 FFI 结构体布局约定，未在其他架构上验证

### 隐私说明

> **注意**：运行时以下信息会显示在 OLED 屏幕上，任何有物理访问的人可以看到：
> - **SSH 登录用户名**（如 `"SSH pi"`）
> - **内网 IP 地址**（如 `"eth0 192.168.1.100"`）
> - USB 设备名称、系统温度/内存/磁盘使用率
>
> 在家用/个人环境中通常无问题,若部署在共享办公、实验室或公共区域，请评估物理窥视风险
