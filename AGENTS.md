# AGENTS.md

## 项目

批量图片处理桌面工具：批量加水印、批量压缩画质。

- 平台：macOS + Windows。必须是可安装的原生桌面应用，**不要**做成 Web 项目或本地服务器方案。
- 界面语言：仅中文。
- 技术栈：Tauri 2（Rust 后端）+ Vite 前端；依赖只用开源组件。
- 状态：功能已实现（批量加水印、批量压缩画质），可构建安装包。

## 环境（本机已就绪）

- rustc / cargo 1.98、node 24、pnpm 10；无 bun。
- 本机 Rust 必须 ≥1.85：tauri 依赖已用 `edition2024`，旧版 cargo 会报 `feature edition2024 is required`（曾用 rustup 从 1.83 升级到 1.98）。
- 包管理器统一用 pnpm，不要混用 npm/yarn 产生多个锁文件。

## 常用命令（项目根目录）

- `pnpm install` / `pnpm tauri dev` / `pnpm tauri build`
- Rust 侧改动提交前：`cd src-tauri && cargo fmt && cargo clippy && cargo test`（`cargo test` 含 4 个 image_ops 单元测试）
- 前端单独构建检查：`pnpm build`（tsc + vite）

## 发布构建

- 本地 macOS 只能构建 macOS 安装包；Windows 安装包由 GitHub Actions 构建（`.github/workflows/build.yml`，push main 或手动触发，产物在 Actions 的 artifacts 里）。
- 不要尝试在 macOS 上交叉编译 Windows 版（Tauri 官方不支持，易失败）。

## 磁盘注意

- 项目位于 30GB 移动盘 `/Volumes/SSD`，曾因空间耗尽构建失败（`No space left on device`）。
- `src-tauri/target` 约 1.4–2GB；空间紧张时 `rm -rf src-tauri/target/debug` 即可释放（测试/clippy 用，可重建），不要动 release 产物。

## 图标

- 唯一图标源文件：`8103d569efbbc798c96af46d99388f1a.jpg`（1080×1080 JPEG）。勿改名、勿删除、勿当作临时文件清理。
- `tauri icon` 只接受 PNG/SVG，先转换再生成（本机验证可用）：
  `sips -s format png 8103d569efbbc798c96af46d99388f1a.jpg --out icon-src.png`
  `pnpm tauri icon icon-src.png` → 生成 `src-tauri/icons/`（`icon-src.png` 已加入 .gitignore，用完可删）
- `._*` 文件是 macOS AppleDouble 元数据，不要提交、不要当图片处理。

## 架构

- 前端无框架：`index.html` + `src/main.ts`（原生 TS）+ `src/styles.css`，两个标签页（批量加水印 / 压缩画质）。
- Rust 侧：`src-tauri/src/lib.rs` 只做命令注册与进度事件转发；图片处理逻辑全在 `src-tauri/src/image_ops.rs`。
- 已实现命令：`add_watermark`、`compress_images`；选项结构体带 `#[serde(rename_all = "camelCase")]`，前端传 camelCase 字段。
- 处理进度通过 Tauri 事件 `progress` 推送（payload: `done`/`total`/`current`）。
- 图片处理全部放 Rust 侧（`image`/`imageproc`/`ab_glyph`），前端只传文件路径；不要把图片 base64 后经 IPC 交给前端处理，批量大图会卡死且占内存。
- 文件/目录选择用 `tauri-plugin-dialog`（Rust 与 npm 两侧已接好，capabilities 已含 `dialog:default`）。
- 文字水印从系统字体加载（macOS PingFang、Windows 微软雅黑等，见 `find_font`），未内嵌字体文件。
