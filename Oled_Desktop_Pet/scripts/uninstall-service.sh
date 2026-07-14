#!/usr/bin/env bash
# 卸载 OLED 桌宠系统服务
#
# 用法: sudo ./scripts/uninstall-service.sh

set -euo pipefail

SERVICE_NAME="oled-pet"
BIN_PATH="/usr/local/bin/oled-pet"
CONFIG_DIR="/etc/oled-pet"
SERVICE_FILE="/etc/systemd/system/${SERVICE_NAME}.service"

if [ "$(id -u)" -ne 0 ]; then
    echo "请使用 sudo 运行: sudo $0"
    exit 1
fi

echo "=== OLED 桌宠 — 卸载系统服务 ==="

# 1. 停止并禁用
if systemctl is-enabled "$SERVICE_NAME" &>/dev/null 2>&1; then
    echo "[1/4] 停止并禁用服务..."
    systemctl disable "$SERVICE_NAME" --now
else
    echo "[1/4] 服务未启用，跳过"
fi

# 2. 删除 service 文件
if [ -f "$SERVICE_FILE" ]; then
    echo "[2/4] 删除 service 文件..."
    rm "$SERVICE_FILE"
    systemctl daemon-reload
else
    echo "[2/4] 文件不存在，跳过"
fi

# 3. 删除二进制
if [ -f "$BIN_PATH" ]; then
    echo "[3/4] 删除二进制..."
    rm "$BIN_PATH"
else
    echo "[3/4] 二进制不存在，跳过"
fi

# 4. 配置目录（询问）
if [ -d "$CONFIG_DIR" ]; then
    echo "[4/4] 配置目录: $CONFIG_DIR"
    read -rp "  删除？[y/N] " ans
    if [ "$ans" = "y" ] || [ "$ans" = "Y" ]; then
        rm -rf "$CONFIG_DIR"
        echo "   → 已删除"
    else
        echo "   → 保留"
    fi
else
    echo "[4/4] 配置目录不存在"
fi

echo ""
echo "=== 卸载完成 ==="
