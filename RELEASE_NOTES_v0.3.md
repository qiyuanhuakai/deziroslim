# Release Notes — tiez-slim v0.3.0

**Release Date**: 2026-06-08

## What's New

tiez-slim v0.3.0 brings 9 major features across privacy, automation, sync, and developer tooling.

### Highlights

| Feature | Description |
|---------|-------------|
| **App Blacklist + Private Mode** | Skip clipboard capture in specified apps (wildcard matching); one-key private mode toggle |
| **Primary Selection** | Track X11 mouse selections alongside clipboard; middle-click paste support |
| **Regex Actions** | Pattern-match clipboard content → auto-run commands; toolbar, context menu, auto-trigger |
| **Export/Import + Backup** | JSON export/import with dedup; auto-backup on app close with retention |
| **Fuzzy Search** | Typo-tolerant, relevance-ranked search with character highlighting |
| **Database Encryption** | AES-256-GCM for sensitive entries; system keyring key management (opt-in) |
| **KDE Connect Sync** | Bidirectional clipboard sync with Android (opt-in) |
| **CLI (tiez-cli)** | 8 subcommands for scripting; Unix socket IPC; Sway/Hyprland integration |
| **i18n** | Full zh-CN + en-US support, 752 keys, 100% coverage |

### Opt-in Features

Two features require compile-time feature flags:

```bash
# Database encryption (AES-256-GCM + system keyring)
cargo build --features secure_storage

# KDE Connect sync (tokio + kdeconnect-proto + mDNS)
cargo build --features kde_connect

# Both
cargo build --features "secure_storage,kde_connect"
```

## Migration Guide (from myclipboard / tiez-clipboard)

If migrating from the old `myclipboard` or `tiez-clipboard` (React + Tauri) version:

1. **Database**: tiez-slim automatically reads the old `myclipboard` database location on first launch and migrates data. No manual export/import needed.
2. **Settings**: Old preferences are carried over where field names match. New fields use sensible defaults.
3. **Hotkeys**: Global hotkey registration may conflict if the old app is still running. Close the old app first.
4. **Features not yet available**: Snippet templates (#9) are planned for v0.4. Whitelist mode is deferred to v1.1.

## CLI Quick Reference

```bash
tiez-cli list [--limit N] [--type text] [--tag work]
tiez-cli search "query"
tiez-cli paste 42 [--rich]
tiez-cli pin 42 [--unpin]
tiez-cli tag 42 work important
tiez-cli delete 42
tiez-cli add "text"
tiez-cli status [--json]
```

## Known Limitations

- **Snippets**: Not yet implemented (planned for v0.4)
- **Whitelist mode**: Deferred to v1.1 (blacklist mode works)
- **Wayland**: Primary Selection tracking requires X11/XFixes; Wayland support is roadmap
- **KDE Connect**: Requires both devices on the same LAN; Android 10+ restricts background clipboard access
- **Encryption**: Requires system keyring (GNOME Keyring or KWallet); unavailable in headless/SSH sessions

## Stats

- **i18n keys**: 752 (zh-CN 100% | en-US 100%)
- **Settings panels**: 10 implemented
- **CLI subcommands**: 8
- **Unit tests**: 203+
- **New modules**: actions, blacklist, clipboard (primary), encryption, export, ipc, search, sync

## Commits

This release includes 31 tasks (T0a through T31) across 4 phases.
