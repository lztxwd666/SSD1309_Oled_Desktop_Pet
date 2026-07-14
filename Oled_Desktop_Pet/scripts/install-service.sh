#!/usr/bin/env bash
# 安装 OLED 桌宠为 systemd 系统服务（开机自启）
#
# 自动探测 OLED 所在的 I2C 总线，适配不同设备。
# 用法: ./scripts/install-service.sh
# 卸载: sudo ./scripts/uninstall-service.sh

set -euo pipefail

SERVICE_NAME="oled-pet"
BIN_PATH="/usr/local/bin/oled-pet"
CONFIG_DIR="/etc/oled-pet"
SERVICE_DIR="/etc/systemd/system"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
CRATE_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
WS_ROOT="$(cd "$CRATE_DIR/.." && pwd)"
SELF="$SCRIPT_DIR/$(basename "$0")"

# ── 编译（非 root 用户执行）─────────────────────────
if [ "$(id -u)" -ne 0 ]; then
    echo "=== OLED 桌宠 — 安装系统服务 ==="
    echo ""
    echo "[1/6] 编译 release 二进制..."
    cd "$CRATE_DIR"
    cargo build --release
    ls -lh "$WS_ROOT/target/release/Oled_Desktop_Pet"
    echo ""
    echo "[!] 后续步骤需要 root 权限"
    exec sudo bash "$SELF"
    exit 0
fi

# ── 以下全部以 root 执行 ────────────────────────────

# 2. 探测 OLED 所在的 I2C 总线
echo ""
echo "[2/6] 探测 OLED 设备 (addr 0x3C)..."
I2C_BUS=""
OLED_ADDR="${OLED_ADDR:-0x3C}"
for bus in /dev/i2c-*; do
    bus_num="${bus#/dev/i2c-}"
    if command -v i2cdetect &>/dev/null; then
        if i2cdetect -y "$bus_num" 0x3C 0x3C 2>/dev/null | grep -qi "3c"; then
            I2C_BUS="$bus_num"
            I2C_DEV="$bus"
            echo "   → 在 /dev/i2c-$I2C_BUS 检测到 OLED"
            break
        fi
    else
        # 回退：尝试读 settings.toml 中的配置
        CFG="$CRATE_DIR/configs/settings.toml"
        I2C_BUS=$(grep 'i2c_bus' "$CFG" 2>/dev/null | grep -o '[0-9]\+' | head -1 || echo "")
        if [ -n "$I2C_BUS" ]; then
            I2C_DEV="/dev/i2c-$I2C_BUS"
            echo "   → 使用配置文件指定: $I2C_DEV"
            break
        fi
    fi
done
if [ -z "$I2C_BUS" ]; then
    # 最终回退：第一个存在的 I2C 设备
    for bus in /dev/i2c-*; do
        I2C_DEV="$bus"
        I2C_BUS="${bus#/dev/i2c-}"
        echo "   → 未检测到 OLED，回退到 $I2C_DEV"
        break
    done
fi
if [ -z "$I2C_BUS" ]; then
    echo "   ✗ 错误: 系统未检测到任何 I2C 设备"
    echo "     请确认 /boot/firmware/config.txt 中已启用:"
    echo "       dtparam=i2c_arm=on"
    exit 1
fi

# 3. 安装二进制
echo "[3/6] 安装二进制..."
BIN_SRC="$WS_ROOT/target/release/Oled_Desktop_Pet"
if [ ! -f "$BIN_SRC" ]; then
    echo "   ✗ 错误: $BIN_SRC 不存在，请先编译"
    exit 1
fi
# 如果旧服务正在运行，先停止再覆盖
systemctl stop "$SERVICE_NAME" 2>/dev/null || true
cp "$BIN_SRC" "$BIN_PATH"
chmod 755 "$BIN_PATH"
echo "   → $BIN_PATH"

# 4. 安装配置
echo "[4/6] 安装配置文件..."
mkdir -p "$CONFIG_DIR/configs" "$CONFIG_DIR/assets/images"
for f in settings.toml font.txt; do
    [ -f "$CONFIG_DIR/configs/$f" ] || cp "$CRATE_DIR/configs/$f" "$CONFIG_DIR/configs/" 2>/dev/null || true
done
[ -f "$CONFIG_DIR/assets/images/ferris.png" ] || cp "$CRATE_DIR/assets/images/ferris.png" "$CONFIG_DIR/assets/images/" 2>/dev/null || true
echo "   → $CONFIG_DIR"

# 5. 生成并安装 systemd service（动态填入 I2C 设备路径）
echo "[5/6] 安装 systemd 服务..."
cat > "$SERVICE_DIR/$SERVICE_NAME.service" << SERVICEOF
[Unit]
Description=OLED Desktop Pet — system monitor on 128x64 I2C OLED
After=multi-user.target
Wants=multi-user.target

[Service]
Type=simple
ExecStart=$BIN_PATH
WorkingDirectory=$CONFIG_DIR
Restart=on-failure
RestartSec=5
StandardOutput=journal
StandardError=journal
SyslogIdentifier=$SERVICE_NAME

# 安全加固
NoNewPrivileges=yes
PrivateTmp=yes
ProtectSystem=strict
ProtectHome=yes
ReadWritePaths=$I2C_DEV $CONFIG_DIR
DeviceAllow=$I2C_DEV rw

# 环境
Environment=RUST_BACKTRACE=0

[Install]
WantedBy=multi-user.target
SERVICEOF
systemctl daemon-reload
echo "   → $SERVICE_DIR/$SERVICE_NAME.service (I2C: $I2C_DEV)"

# 6. 启用并启动
echo "[6/6] 启用开机自启..."
systemctl enable "$SERVICE_NAME" --now
sleep 1
systemctl status "$SERVICE_NAME" --no-pager --lines=0 || true

echo ""
echo "=== 安装完成 ==="
echo "  I2C 设备: $I2C_DEV"
echo "  查看日志: journalctl -u $SERVICE_NAME -f"
echo "  停止:     systemctl stop $SERVICE_NAME"
echo "  卸载:     sudo $SCRIPT_DIR/uninstall-service.sh"
