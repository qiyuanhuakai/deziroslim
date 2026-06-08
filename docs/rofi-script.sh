#!/usr/bin/env bash
# rofi-script.sh — Rofi integration for tiez-slim clipboard manager.
#
# Usage:
#   ~/.config/tiez/rofi-script.sh
#
# Prerequisites:
#   - tiez-slim running in the background
#   - tiez-cli in PATH (or edit TIEZ_CLI below)
#   - rofi installed
#
# This script shows clipboard history in rofi. Selecting an entry copies it
# to the clipboard via tiez-cli paste.

set -euo pipefail

# ── Configuration ──────────────────────────────────────────────────────

TIEZ_CLI="${TIEZ_CLI:-tiez-cli}"
ROFI="${ROFI:-rofi}"

# ── Preflight checks ──────────────────────────────────────────────────

if ! command -v "$TIEZ_CLI" &>/dev/null; then
    "$ROFI" -e "tiez-cli not found. Install tiez-slim or set TIEZ_CLI." 2>/dev/null \
        || echo "Error: tiez-cli not found. Install tiez-slim or set TIEZ_CLI." >&2
    exit 1
fi

if ! "$TIEZ_CLI" status &>/dev/null; then
    "$ROFI" -e "tiez-slim is not running. Start it first." 2>/dev/null \
        || echo "Error: tiez-slim is not running. Start it first." >&2
    exit 1
fi

# ── Fetch entries ──────────────────────────────────────────────────────

entries_json=$("$TIEZ_CLI" list --json 2>/dev/null) || {
    "$ROFI" -e "Failed to fetch clipboard history." 2>/dev/null \
        || echo "Error: Failed to fetch clipboard history." >&2
    exit 1
}

if [ -z "$entries_json" ] || [ "$entries_json" = "[]" ]; then
    "$ROFI" -e "Clipboard history is empty." 2>/dev/null \
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

entry_id=$(echo "$selection" | grep -oP '#\K[0-9]+$' || true)

if [ -z "$entry_id" ]; then
    "$ROFI" -e "Could not parse entry ID from selection." 2>/dev/null \
        || echo "Error: Could not parse entry ID from selection." >&2
    exit 1
fi

"$TIEZ_CLI" paste "$entry_id"
