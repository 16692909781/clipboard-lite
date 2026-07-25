# Clipboard Lite

轻量本地剪贴板历史管理工具，基于 Tauri 2 + Vue 3 + TypeScript + Rust。核心功能开源免费，Pro 功能通过离线授权后的动态插件扩展。

## 当前功能

- 本地剪贴板监听：文本、图片、文件路径
- SQLite 历史记录：去重、收藏、搜索、容量上限、保存时长
- 全局快捷键：默认 `Ctrl+Shift+V`
- 主面板：搜索、键盘选择、回车粘贴、右键收藏/删除
- 设置页：开机自启、最大记录数、快捷键、主题、忽略程序、离线激活
- 系统托盘：打开面板、设置、暂停/恢复监听、退出
- Pro 框架：离线 license、设备指纹、动态库 ABI 占位

## 隐私边界

Clipboard Lite 不包含云端服务、埋点、广告 SDK 或主动网络请求。历史记录、配置和授权文件都保存在系统应用数据目录。

Windows 默认路径：

```text
%APPDATA%\com.clipboardlite.desktop\
```

## 环境要求

- Node.js 20+
- Rust 1.77+
- Windows 10/11
- Visual Studio Build Tools，需安装 C++ 桌面开发和 Windows SDK
- WebView2 Runtime

## 开发

```bash
npm install
npm run tauri dev
```

PowerShell 如果拦截 `npm.ps1`，可以使用：

```bash
npm.cmd run tauri dev
```

## 构建

```bash
npm run tauri build
```

输出：

- NSIS 安装包：`src-tauri/target/release/bundle/nsis/`
- 便携版主程序：`src-tauri/target/release/clipboard-lite.exe`

Tauri 2 当前没有 `portable` bundle target，因此便携版使用 release 可执行文件交付。

## 目录结构

```text
clipboard-lite/
├── src/                      # Vue 前端开源源码
│   ├── components/
│   ├── views/
│   ├── stores/
│   ├── utils/
│   ├── App.vue
│   └── main.ts
├── src-tauri/                # Rust 后端开源源码
│   ├── src/
│   │   ├── clipboard.rs
│   │   ├── storage.rs
│   │   ├── hotkey.rs
│   │   ├── plugin_loader.rs
│   │   ├── license.rs
│   │   └── main.rs
│   └── Cargo.toml
├── plugins/                  # Pro 插件占位目录，不提交动态库
├── docs/
└── README.md
```

## 文档

- [架构说明](docs/ARCHITECTURE.md)
- [Pro 插件 API](docs/PLUGIN_API.md)
- [Windows 构建说明](docs/BUILDING_WINDOWS.md)
- [项目计划](docs/PROJECT_PLAN.md)

## 许可证

核心代码使用 MIT License。Pro 插件可在单独私有仓库闭源分发。
