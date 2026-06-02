# MyClipboard

Rust 原生版剪贴板管理器，迁移自 `../tiez-clipboard` 的核心能力，目标是去掉 React/Tauri/WebView 带来的额外开销。

## 当前实现

- 原生自绘 UI：`eframe/egui`，无系统标题栏，自绘 TieZ 顶栏可拖拽，支持圆角窗口和可切换应用边框；工具栏图标使用 egui 原生矢量绘制以避免小尺寸 SVG 模糊
- 字体优先使用系统 `Maple Mono NF CN`，并回退到 Noto/思源/WenQuanYi 等 CJK 字体
- Linux 剪贴板：`arboard` 轮询监听文本、富文本 HTML、图片和文件列表；文本自动识别 URL、代码、文件路径、图片/视频 data URL 等类型，图片会保存为 PNG data URL 并可写回系统剪贴板，历史列表可显示图片缩略图，文件条目按文件名/目录摘要展示，文件列表按系统 file-list target 写回
- 持久化：`rusqlite` + bundled SQLite
- 历史能力：搜索、类型过滤、标签过滤、置顶、删除、清空、标签编辑、左键/右键/Enter 按 TieZ 语义复制并粘贴
- TieZ 风格主界面：380×680 竖向剪贴板浮窗、小尺寸工具按钮、紧凑标签胶囊、单列历史流、类型徽标、敏感内容遮罩、左/右/上三向贴边边条隐藏
- 表情包页面：顶部笑脸按钮进入 `表情包` 全页，支持 EMOJI/收藏 Tab，内置 TieZ 常用 emoji 分组，Tab 状态随设置保存
- 设置页面：顶部齿轮按钮进入全页设置，按 TieZ 分为常规设置、快捷键设置、剪贴板设置、界面设置、默认打开程序、过滤/标签目录、数据管理和平台能力；已接通项即时生效并持久化
- 标签目录：可新增/移除常用标签，点击目录标签可快速加入当前条目；移除目录标签不会删除既有条目标签
- 辅助能力：敏感内容启发式识别（token/password/email/长数字等）、`sensitive`/`密码` 标签遮罩、SQLite 设置表保存 UI 偏好
- Linux 平台能力：X11 `_NET_ACTIVE_WINDOW` 前台窗口标题识别、录制式自定义全局快捷键（含鼠标中键）、StatusNotifierItem 系统托盘、窗口置顶、跟随鼠标呼出、四向边缘隐藏停靠、最小化/关闭到托盘、可配置 `xdotool` 粘贴方式、XDG `.desktop` 自动应用下拉、`open` 调用系统/指定默认应用
- 数据约束：最多保留 1000 条，非置顶条目超过 30 天清理
- 平台预留：`src/platform/` 按 OS 分层；Windows 目前提供占位实现

## 构建与运行

```bash
cargo run
cargo run -- --db-path /path/to/clipboard.db
MYCLIPBOARD_DB_PATH=/path/to/clipboard.db cargo run
cargo test
cargo build --release
```

GUI 调试模式：

```bash
cargo run -- --dev
# 或
MYCLIPBOARD_DEV=1 cargo run
# 或编译期启用
cargo run --features devtools
```

dev 模式会在窗口内显示原生调试面板，包含剪贴板事件数、保存成功数、错误数、当前搜索、选中条目和最近状态，便于调试 GUI 交互与剪贴板监听链路。
debug 构建下还会启用 egui 布局覆盖层开关；release 构建中保留状态计数、Inspection 和 Memory 面板。

设置页面位于顶部矢量齿轮按钮。已接通的切换项会立即生效并自动保存到 SQLite；搜索框可在设置中隐藏，类型/标签胶囊过滤条会随搜索区一起隐藏。顶部笑脸是表情包入口，不再用于显示/隐藏敏感内容。历史项左键会写入剪贴板并粘贴，右键会尽量带格式写入并粘贴，方向键选择后按 Enter 走同一粘贴流程；`粘贴后删除` 优先于 `粘贴后移到顶部`。快捷键设置为录制模式，支持主热键多条、顺序粘贴、富文本粘贴、搜索聚焦和 `MouseMiddle`；粘贴模拟方式可选 Shift+Insert、Ctrl+V 或逐字输入；默认打开程序会自动扫描 XDG 应用并用下拉框选择。若托盘可用，可开启“关闭按钮隐藏到托盘”，真正退出仍可从托盘菜单执行。

Linux 需要图形环境。当前优先支持 X11；全局键盘快捷键使用 X11 `grab_key` 注册，鼠标中键使用 `grab_button(Button2)` 注册，跟随鼠标与边缘停靠使用 X11 `query_pointer` + egui `ViewportCommand::OuterPosition`。粘贴模拟使用 `xdotool`，因此运行环境需安装 `xdotool`。文件列表粘贴在 Linux 上优先使用 `xclip` 写入 `x-special/gnome-copied-files` 以兼容 Nautilus/GTK 文件管理器，并保留 `arboard` URI-list 兜底。Wayland 下全局热键、鼠标定位与模拟粘贴仍取决于桌面环境/portal/evdev 权限。`arboard` 在 Linux 上可读写文本、HTML、图片和文件列表；Linux 剪贴板由最后写入者进程托管，本应用作为常驻进程负责持有 clipboard owner。系统托盘使用 `ksni`/D-Bus StatusNotifierItem，不依赖 GTK；是否显示取决于桌面环境是否提供 SNI/AppIndicator 托盘区域。数据库默认位于 XDG 数据目录，也可通过 `--db-path`、`MYCLIPBOARD_DB_PATH` 或设置页保存重启后路径来配置。egui 的 widget ID 冲突警告和调试红框默认被禁用，避免调试标记污染正式界面。

## 与旧版差异

旧项目使用 React + Tauri 2 + WebView。此版本对齐其主界面视觉和核心数据模型，并已用 Rust 原生能力补齐文本/富文本/图片/文件剪贴板、X11 全局呼出、鼠标中键、点击/键盘粘贴流程、系统托盘、边缘停靠、默认打开应用设置和可配置数据位置；WebView 专属的 HTML 渲染快照、复杂顺序粘贴队列和加密仍作为后续原生能力继续推进。
