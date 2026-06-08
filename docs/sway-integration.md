# Sway / Hyprland / i3 集成指南 / Integration Guide

## 前提条件

`tiez-slim` 必须在后台运行，`tiez-cli` 才能正常工作。窗口管理器配置中应先启动 `tiez-slim`，再使用 `tiez-cli` 进行脚本集成。

`tiez-slim` must be running in the background for `tiez-cli` to work. Start `tiez-slim` in your window manager config before using `tiez-cli` for script integration.

## 安装 rofi 脚本

将 `rofi-script.sh` 复制到你的配置目录：

```bash
mkdir -p ~/.config/tiez
cp docs/rofi-script.sh ~/.config/tiez/rofi-script.sh
chmod +x ~/.config/tiez/rofi-script.sh
```

## Sway 配置

在 `~/.config/sway/config` 中添加：

```text
# 启动 tiez-slim
exec tiez-slim

# 剪贴板历史快捷键 (Mod+V)
bindsym $mod+v exec ~/.config/tiez/rofi-script.sh
```

## i3 配置

在 `~/.config/i3/config` 中添加（语法与 Sway 相同）：

```text
# 启动 tiez-slim
exec tiez-slim

# 剪贴板历史快捷键 (Mod+V)
bindsym $mod+v exec ~/.config/tiez/rofi-script.sh
```

## Hyprland 配置

在 `~/.config/hypr/hyprland.conf` 中添加：

```text
# 启动 tiez-slim
exec-once = tiez-slim

# 剪贴板历史快捷键 (Super+V)
bind = $mainMod, V, exec, ~/.config/tiez/rofi-script.sh
```

## wofi 替代方案

如果使用 wofi 而非 rofi，修改脚本中的 `ROFI` 变量：

```bash
export ROFI=wofi
```

或在 Sway 配置中直接指定：

```text
bindsym $mod+v exec ROFI=wofi ~/.config/tiez/rofi-script.sh
```

## 直接使用 tiez-cli

不依赖 rofi/wofi，直接在终端或脚本中使用 `tiez-cli`：

```bash
# 列出最近 20 条记录
tiez-cli list --limit 20

# 按类型过滤（只显示 URL）
tiez-cli list --type url

# 搜索剪贴板历史
tiez-cli search "关键词"

# 复制指定条目到剪贴板
tiez-cli paste 42

# 查看服务器状态
tiez-cli status

# 添加新条目
tiez-cli add "要添加的文本"

# JSON 格式输出（适合脚本处理）
tiez-cli --json list | jq '.[].preview'
```

## dmenu 集成示例

使用 dmenu 而非 rofi 的简易脚本：

```bash
#!/usr/bin/env bash
selection=$(tiez-cli list --json | python3 -c "
import sys, json
for e in json.load(sys.stdin):
    preview = e['preview'].replace('\n', ' ')[:80]
    print(f\"#{e['id']}  {preview}\")
" | dmenu -i -p "Clipboard:" -l 15)

[ -z "$selection" ] && exit 0
entry_id=$(echo "$selection" | grep -oP '#\K[0-9]+')
tiez-cli paste "$entry_id"
```

## 注意事项

- `tiez-cli` 通过 Unix socket 与 `tiez-slim` 通信，socket 路径为 `$XDG_RUNTIME_DIR/tiez-slim-linux.sock`
- 如果 `tiez-slim` 未运行，`tiez-cli` 会报连接错误（exit code 2）
- `rofi-script.sh` 内置了连接检查，会通过 rofi 弹窗提示错误信息
- Wayland 下粘贴行为取决于你的 compositor 设置，`tiez-cli paste` 仅写入剪贴板，不自动粘贴到活动窗口

## Notes

- `tiez-cli` communicates with `tiez-slim` over a Unix domain socket at `$XDG_RUNTIME_DIR/tiez-slim-linux.sock`
- If `tiez-slim` is not running, `tiez-cli` returns exit code 2 (connection refused)
- `rofi-script.sh` checks the connection and shows an error popup via rofi if tiez-slim is down
- Under Wayland, paste behavior depends on your compositor. `tiez-cli paste` writes to the clipboard only; it does not auto-paste into the active window.
