# Sway / Hyprland / i3 集成指南 / Integration Guide

## 前提条件

`deziroslim` 必须在后台运行，`dzc-slim` 才能正常工作。窗口管理器配置中应先启动 `deziroslim`，再使用 `dzc-slim` 进行脚本集成。

`deziroslim` must be running in the background for `dzc-slim` to work. Start `deziroslim` in your window manager config before using `dzc-slim` for script integration.

**Note:** 集成示例使用 `python3` 解析 JSON，请确保系统已安装 Python 3。 / The integration examples use `python3` for JSON parsing. Ensure Python 3 is installed.

## 安装 rofi 脚本

将 `rofi-script.sh` 复制到你的配置目录：

```bash
mkdir -p ~/.config/deziroslim
cp docs/rofi-script.sh ~/.config/deziroslim/rofi-script.sh
chmod +x ~/.config/deziroslim/rofi-script.sh
```

## Sway 配置

在 `~/.config/sway/config` 中添加：

```text
# 启动 deziroslim
exec deziroslim

# 剪贴板历史快捷键 (Mod+V)
bindsym $mod+v exec ~/.config/deziroslim/rofi-script.sh
```

## i3 配置

在 `~/.config/i3/config` 中添加（语法与 Sway 相同）：

```text
# 启动 deziroslim
exec deziroslim

# 剪贴板历史快捷键 (Mod+V)
bindsym $mod+v exec ~/.config/deziroslim/rofi-script.sh
```

## Hyprland 配置

在 `~/.config/hypr/hyprland.conf` 中添加：

```text
# 启动 deziroslim
exec-once = deziroslim

# 剪贴板历史快捷键 (Super+V)
bind = $mainMod, V, exec, ~/.config/deziroslim/rofi-script.sh
```

## wofi 替代方案

如果使用 wofi 而非 rofi，修改脚本中的 `ROFI` 变量：

```bash
export ROFI=wofi
```

或在 Sway 配置中直接指定：

```text
bindsym $mod+v exec ROFI=wofi ~/.config/deziroslim/rofi-script.sh
```

## 直接使用 dzc-slim

不依赖 rofi/wofi，直接在终端或脚本中使用 `dzc-slim`：

```bash
# 列出最近 20 条记录
dzc-slim list --limit 20

# 按类型过滤（只显示 URL）
dzc-slim list --type url

# 搜索剪贴板历史
dzc-slim search "关键词"

# 复制指定条目到剪贴板
dzc-slim paste 42

# 查看服务器状态
dzc-slim status

# 添加新条目
dzc-slim add "要添加的文本"

# JSON 格式输出（适合脚本处理）
dzc-slim --json list | jq '.[].preview'
```

## dmenu 集成示例

使用 dmenu 而非 rofi 的简易脚本：

```bash
#!/usr/bin/env bash
selection=$(dzc-slim list --json | python3 -c "
import sys, json
for e in json.load(sys.stdin):
    preview = e['preview'].replace('\n', ' ')[:80]
    print(f\"#{e['id']}  {preview}\")
" | dmenu -i -p "Clipboard:" -l 15)

[ -z "$selection" ] && exit 0
entry_id=$(echo "$selection" | sed -n 's/.*#\([0-9]\+\)$/\1/p')
dzc-slim paste "$entry_id"
```

## 注意事项

- `dzc-slim` 通过 Unix socket 与 `deziroslim` 通信，socket 路径为 `$XDG_RUNTIME_DIR/deziroslim.sock`
- 如果 `deziroslim` 未运行，`dzc-slim` 会报连接错误（exit code 2）
- `rofi-script.sh` 内置了连接检查，会通过 rofi 弹窗提示错误信息
- Wayland 下粘贴行为取决于你的 compositor 设置，`dzc-slim paste` 仅写入剪贴板，不自动粘贴到活动窗口

## Notes

- `dzc-slim` communicates with `deziroslim` over a Unix domain socket at `$XDG_RUNTIME_DIR/deziroslim.sock`
- If `deziroslim` is not running, `dzc-slim` returns exit code 2 (connection refused)
- `rofi-script.sh` checks the connection and shows an error popup via rofi if deziroslim is down
- Under Wayland, paste behavior depends on your compositor. `dzc-slim paste` writes to the clipboard only; it does not auto-paste into the active window.
