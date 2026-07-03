#!/usr/bin/env bash
# rofi-script.sh — Rofi integration for deziroslim clipboard manager.
#
# Usage:
#   ~/.config/deziroslim/rofi-script.sh
#
# Prerequisites:
#   - deziroslim running in the background
#   - dzc-slim in PATH (or edit DZC_SLIM_CLI below)
#   - rofi installed
#   - python3 (for JSON parsing)
#
# This script shows clipboard history in rofi. Selecting an entry copies it
# to the clipboard via dzc-slim paste.

set -euo pipefail

# ── Configuration ──────────────────────────────────────────────────────

DZC_SLIM_CLI="${DZC_SLIM_CLI:-dzc-slim}"
ROFI="${ROFI:-rofi}"

# ── Preflight checks ──────────────────────────────────────────────────

if ! command -v "$DZC_SLIM_CLI" &>/dev/null; then
    "$ROFI" -e "未找到 dzc-slim / dzc-slim not found. 请安装 deziroslim 或设置 DZC_SLIM_CLI / Install deziroslim or set DZC_SLIM_CLI." 2>/dev/null \
        || echo "Error: dzc-slim not found. Install deziroslim or set DZC_SLIM_CLI." >&2
    exit 1
fi

if ! "$DZC_SLIM_CLI" status &>/dev/null; then
    "$ROFI" -e "deziroslim 未运行 / deziroslim is not running. 请先启动它 / Start it first." 2>/dev/null \
        || echo "Error: deziroslim is not running. Start it first." >&2
    exit 1
fi

# ── Fetch entries ──────────────────────────────────────────────────────

entries_json=$("$DZC_SLIM_CLI" list --json 2>/dev/null) || {
    "$ROFI" -e "获取剪贴板历史失败 / Failed to fetch clipboard history." 2>/dev/null \
        || echo "Error: Failed to fetch clipboard history." >&2
    exit 1
}

if [ -z "$entries_json" ] || [ "$entries_json" = "[]" ]; then
    "$ROFI" -e "剪贴板历史为空 / Clipboard history is empty." 2>/dev/null \
        || echo "Clipboard history is empty." >&2
    exit 0
fi

# ── Build rofi menu lines ──────────────────────────────────────────────
# Format: "preview text  (kind)  #ID"
# The ID is extracted after the last # marker on selection.

menu=$(echo "$entries_json" | python3 -c "
import sys, json
entries = json.load(sys.stdin)
for e in entries:
    eid = e.get('id', '')
    kind = e.get('kind', 'text')
    preview = e.get('preview', '').replace('\n', ' ').strip()
    if len(preview) > 100:
        preview = preview[:97] + '...'
    pin = '📌 ' if e.get('is_pinned') else ''
    tags = ''
    if e.get('tags'):
        tags = ' [' + ', '.join(e['tags']) + ']'
    print(f'{pin}{preview}  ({kind}){tags}  #{eid}')
")

# ── Show rofi and capture selection ────────────────────────────────────

selection=$(echo "$menu" | "$ROFI" -dmenu -i -p "Clipboard" -lines 15 -width 60 2>/dev/null) || {
    # User cancelled rofi (Esc or q). Exit silently.
    exit 0
}

if [ -z "$selection" ]; then
    exit 0
fi

# ── Extract entry ID and paste ─────────────────────────────────────────
# ID follows the last '#' in the line.

entry_id=$(echo "$selection" | grep -oE '#[0-9]+$' | sed 's/^#//' || true)

if [ -z "$entry_id" ]; then
    "$ROFI" -e "无法解析条目 ID / Could not parse entry ID from selection." 2>/dev/null \
        || echo "Error: Could not parse entry ID from selection." >&2
    exit 1
fi

"$DZC_SLIM_CLI" paste "$entry_id" || {
    "$ROFI" -e "粘贴失败 / Failed to paste entry." 2>/dev/null \
        || echo "Error: Failed to paste entry." >&2
    exit 1
}
