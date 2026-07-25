# Clipboard Lite — 项目计划

> 轻量本地剪贴板历史工具 · Tauri 2.0 + Vue 3 + TypeScript  
> 架构：开源核心 + Pro 闭源插件 · 纯本地 · 无网络

---

## 1. 项目概述

### 1.1 产品定位

| 维度 | 目标 |
|------|------|
| 运行方式 | 100% 本地，零网络请求 |
| 平台优先级 | Windows MVP → macOS / Linux |
| 体积目标 | 安装包 < 15 MB（Windows NSIS） |
| 内存目标 | 空闲 < 30 MB，面板打开 < 60 MB |
| 启动目标 | 冷启动 < 800 ms（SSD） |
| 开源策略 | 核心功能 MIT/Apache-2.0 开源，Pro 功能闭源插件 |

### 1.2 核心用户流程

```
系统剪贴板变化
    ↓
Rust 监听 + 去重 + 隐私过滤
    ↓
SQLite 持久化（上限 500 条）
    ↓
Ctrl+Shift+V 呼出面板
    ↓
搜索 / 选择 / 回车粘贴
```

### 1.3 技术栈确认

| 层级 | 选型 | 理由 |
|------|------|------|
| 桌面框架 | Tauri 2.0 | 体积小、Rust 后端、系统 API 访问 |
| 前端 | Vue 3 + Vite + TS | Composition API，生态成熟 |
| UI 库 | **Naive UI**（推荐） | 按需引入体积更小；Element Plus 亦可 |
| 状态 | Pinia | 轻量、TS 友好 |
| 存储 | tauri-plugin-sql | 官方 SQLite 插件（Tauri 2 命名） |
| 剪贴板 | arboard + Windows API | 跨平台 + Windows 特化 |
| 快捷键 | tauri-plugin-global-shortcut | 全局热键 |
| 托盘 | tauri-plugin-autostart + 系统托盘 API | 自启 + 后台驻留 |

---

## 2. 目录结构

```
clipboard-lite/
├── src/                          # Vue 前端（开源）
│   ├── components/
│   │   ├── ClipItem.vue          # 单条记录卡片
│   │   ├── SearchBar.vue         # 搜索框
│   │   ├── ContextMenu.vue       # 右键菜单
│   │   └── ThemeSwitch.vue
│   ├── views/
│   │   ├── MainPanel.vue         # 主面板（快捷键呼出）
│   │   ├── Favorites.vue         # 收藏页
│   │   └── Settings.vue          # 设置页
│   ├── stores/
│   │   ├── clipboard.ts          # 剪贴板历史状态
│   │   ├── settings.ts           # 用户配置
│   │   └── license.ts            # Pro 授权状态
│   ├── utils/
│   │   ├── format.ts             # 时间/类型格式化
│   │   └── keyboard.ts           # 键盘导航逻辑
│   ├── App.vue
│   └── main.ts
├── src-tauri/                    # Rust 后端（开源）
│   ├── src/
│   │   ├── lib.rs
│   │   ├── main.rs
│   │   ├── clipboard.rs          # 监听核心
│   │   ├── storage.rs            # SQLite CRUD
│   │   ├── hotkey.rs             # 全局快捷键
│   │   ├── plugin_loader.rs      # Pro 插件加载
│   │   ├── license.rs            # 离线授权校验
│   │   └── commands/             # Tauri IPC 命令
│   │       ├── mod.rs
│   │       ├── clip.rs
│   │       └── settings.rs
│   ├── capabilities/             # Tauri 2 权限声明
│   ├── tauri.conf.json
│   └── Cargo.toml
├── plugins/                      # Pro 插件（闭源，.gitignore）
│   └── pro-plugin.dll
├── docs/
│   ├── PROJECT_PLAN.md           # 本文档
│   ├── ARCHITECTURE.md           # 架构详解（Step 2 后补充）
│   └── PLUGIN_API.md             # Pro 插件接口文档（Step 5 后补充）
├── .gitignore
├── package.json
├── vite.config.ts
├── tsconfig.json
└── README.md
```

---

## 3. 六步实施计划

### Step 1：项目骨架初始化（预估 1 天）

**目标**：`npm run tauri dev` 可启动空白窗口。

#### 任务清单

- [ ] `npm create tauri-app@latest` 选择 Vue + TypeScript 模板
- [ ] 升级/锁定 Tauri 2.x 依赖
- [ ] 安装 Naive UI、Pinia、Vue Router
- [ ] 配置 `tauri.conf.json`：
  - 窗口默认隐藏（`visible: false`）
  - 无边框 + 透明背景（面板浮层效果）
  - 系统托盘图标
- [ ] 添加 Rust 依赖：`tauri-plugin-sql`、`tauri-plugin-global-shortcut`、`tauri-plugin-autostart`
- [ ] 创建上述目录骨架与空模块文件
- [ ] 编写 `README.md` 开发/构建说明

#### 验收标准

```bash
npm install
npm run tauri dev   # 窗口正常弹出，无报错
```

#### 关键配置片段

**tauri.conf.json 窗口策略**：
- 主窗口：`decorations: false`, `alwaysOnTop: true`, `skipTaskbar: true`
- 尺寸：400×500，居中
- 托盘：启用，左键点击呼出面板

---

### Step 2：剪贴板监听 + SQLite 存储（预估 2–3 天）

**目标**：剪贴板变化自动入库，前端可读取列表。

#### 2.1 数据库 Schema

```sql
CREATE TABLE IF NOT EXISTS clips (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    content     TEXT NOT NULL,           -- 文本内容或文件路径 JSON
    content_hash TEXT NOT NULL UNIQUE,   -- SHA256 去重用
    clip_type   TEXT NOT NULL,           -- 'text' | 'image' | 'files'
    preview     TEXT,                    -- 预览文本（图片为 base64 缩略图路径）
    is_favorite INTEGER DEFAULT 0,
    source_app  TEXT,                    -- 来源程序（可选）
    created_at  INTEGER NOT NULL,        -- Unix 毫秒时间戳
    expires_at  INTEGER                  -- 过期时间（可选）
);

CREATE INDEX idx_clips_created ON clips(created_at DESC);
CREATE INDEX idx_clips_favorite ON clips(is_favorite DESC, created_at DESC);
```

#### 2.2 clipboard.rs 核心逻辑

```
┌─────────────────────────────────────┐
│  ClipboardWatcher (后台线程)         │
│  ├─ 轮询间隔：300ms（Windows）       │
│  ├─ 读取：Text / Image / FileList    │
│  ├─ 隐私过滤：                        │
│  │   ├─ 密码字段检测（CF_UNICODETEXT + 窗口类名）│
│  │   └─ 忽略列表（配置项）            │
│  ├─ 去重：content_hash 冲突则 UPDATE  │
│  └─ 上限：超出 max_count 删最旧       │
└─────────────────────────────────────┘
```

#### 2.3 Tauri Commands（IPC 接口）

| Command | 参数 | 返回 | 说明 |
|---------|------|------|------|
| `get_clips` | `{ limit, offset, search }` | `Clip[]` | 分页 + 搜索 |
| `delete_clip` | `{ id }` | `()` | 单条删除 |
| `clear_all_clips` | — | `()` | 清空历史 |
| `toggle_favorite` | `{ id }` | `bool` | 切换收藏 |
| `paste_clip` | `{ id }` | `()` | 写入系统剪贴板并模拟粘贴 |

#### 2.4 事件推送

```rust
// 新记录入库后 emit 到前端
app.emit("clip-added", clip)?;
```

#### 验收标准

- 复制文本/图片/文件路径后，数据库自动新增记录
- 相同内容重复复制不产生重复行，仅更新时间
- 超过 500 条自动删除最旧记录
- 前端 `invoke('get_clips')` 返回正确列表

---

### Step 3：全局快捷键 + 主面板 UI（预估 2–3 天）

**目标**：`Ctrl+Shift+V` 呼出面板，搜索、键盘导航、回车粘贴完整可用。

#### 3.1 快捷键模块 (hotkey.rs)

- 默认注册 `Ctrl+Shift+V`
- 呼出逻辑：显示窗口 → 聚焦搜索框 → 置顶
- 冲突检测：注册失败 emit `hotkey-conflict` 事件
- 设置页可修改，修改后注销旧键再注册新键

#### 3.2 主面板 UI (MainPanel.vue)

```
┌──────────────────────────────────┐
│  🔍 搜索...                        │
├──────────────────────────────────┤
│  ★ 收藏项（置顶）                  │
│  ─────────────────────────────  │
│  📋 文本预览...        2分钟前     │  ← 选中高亮
│  🖼 [图片]              5分钟前     │
│  📁 3 个文件            1小时前    │
├──────────────────────────────────┤
│  ↑↓ 选择  Enter 粘贴  Esc 关闭    │
└──────────────────────────────────┘
```

#### 3.3 键盘交互

| 按键 | 行为 |
|------|------|
| `↑` / `↓` | 移动选中项 |
| `Enter` | 粘贴选中项并隐藏面板 |
| `Esc` | 隐藏面板 |
| 输入文字 | 实时过滤列表 |

#### 3.4 点击外部关闭

- 监听 `window.blur` 或使用 Tauri `set_focus` + 失焦检测
- Windows：可选 `WS_EX_TOOLWINDOW` 避免任务栏图标

#### 3.5 粘贴实现

```rust
// paste_clip 流程
1. 将内容写入系统剪贴板（arboard）
2. 隐藏面板窗口
3. 模拟 Ctrl+V（enigo 或 Windows SendInput）
```

#### 验收标准

- 快捷键全局有效（应用未聚焦时也可呼出）
- 搜索实时过滤，100ms 内响应
- 回车粘贴到上一个活动窗口光标处
- 点击面板外区域自动隐藏

---

### Step 4：收藏 + 设置页（预估 1–2 天）

#### 4.1 收藏功能

- `toggle_favorite` 切换，`get_clips` 查询时 `ORDER BY is_favorite DESC, created_at DESC`
- 主面板顶部固定显示收藏项（最多 10 条）
- 右键菜单：收藏 / 取消收藏 / 删除

#### 4.2 设置页 (Settings.vue)

| 设置项 | 类型 | 默认值 | 存储 |
|--------|------|--------|------|
| 开机自启 | Switch | false | tauri-plugin-autostart |
| 最大记录数 | InputNumber | 500 | config.json |
| 全局快捷键 | HotkeyInput | Ctrl+Shift+V | config.json |
| 主题 | Radio | system | config.json |
| 保存时长 | Select | 30天/永久 | config.json |
| 忽略程序列表 | TagInput | [] | config.json |
| Pro 激活码 | Input | — | license.enc |

#### 4.3 配置文件路径

```
Windows: %APPDATA%\clipboard-lite\config.json
         %APPDATA%\clipboard-lite\license.enc
         %APPDATA%\clipboard-lite\data.db
```

#### 验收标准

- 收藏项置顶且重启后保留
- 修改最大记录数后立即生效（超出部分删除）
- 主题切换即时应用（Naive UI darkTheme）
- 开机自启注册表项正确写入

---

### Step 5：Pro 插件框架 + 离线授权（预估 2–3 天）

#### 5.1 插件接口 (plugin_loader.rs)

```rust
/// Pro 插件必须实现的 C ABI 接口
#[repr(C)]
pub struct PluginInfo {
    pub name: *const c_char,
    pub version: *const c_char,
}

#[repr(C)]
pub struct PluginVTable {
    pub get_info: extern "C" fn() -> PluginInfo,
    pub on_load: extern "C" fn(app: AppHandle) -> i32,
    pub on_unload: extern "C" fn() -> i32,
    pub get_menu_items: extern "C" fn() -> *const MenuItem,
    pub get_settings_pages: extern "C" fn() -> *const SettingsPage,
}

// 动态加载
pub fn load_pro_plugin(path: &Path) -> Result<Library, PluginError> {
    let lib = unsafe { Library::new(path)? };
    let vtable: Symbol<PluginVTable> = unsafe { lib.get(b"plugin_vtable")? };
    // ...
}
```

#### 5.2 离线授权 (license.rs)

```
激活码格式：XXXX-XXXX-XXXX-XXXX（32 位有效载荷 + 校验）

校验流程：
1. 读取本地 license.enc（AES-256-GCM 加密）
2. 解密得到 { activation_code, device_ids[], activated_at }
3. 生成当前设备指纹：
   - CPU ID + 主板序列号 + 磁盘序列号 → SHA256
4. 验证：
   - 激活码 HMAC 签名合法
   - 当前设备指纹在 device_ids 中，或 device_ids.len() < 2
5. 通过 → load_pro_plugin()；失败 → 跳过，隐藏 Pro 入口
```

#### 5.3 Pro 功能预留接口

| 功能 | 接口 | MVP 状态 |
|------|------|----------|
| 加密存储 | `encrypt_clip(content, password)` | 接口预留 |
| 快捷短语 | `expand_snippet(trigger)` | 接口预留 |
| 导入导出 | `export_clips(path)` / `import_clips(path)` | 接口预留 |
| 纯文本粘贴 | `paste_as_plain_text(id)` | 接口预留 |

#### 5.4 前端 Pro 入口

- 设置页底部「激活 Pro」输入框
- 授权成功后显示 Pro 菜单项（由插件注册）
- 未授权时不渲染 Pro 相关 UI

#### 验收标准

- 无授权时 Pro 入口不可见，插件不加载
- 输入有效激活码后 Pro 菜单出现
- 第二台设备激活成功，第三台被拒绝
- 插件 DLL 缺失时不崩溃，降级提示

---

### Step 6：系统托盘 + 打包 + 异常处理（预估 1–2 天）

#### 6.1 系统托盘

- 托盘图标：应用启动后隐藏主窗口，仅显示托盘
- 左键单击：呼出/隐藏主面板
- 右键菜单：打开面板 / 设置 / 暂停监听 / 退出

#### 6.2 打包配置

```json
// tauri.conf.json bundle 配置
{
  "bundle": {
    "targets": ["nsis"],
    "windows": {
      "nsis": {
        "installMode": "both",
        "languages": ["SimpChinese", "English"]
      }
    }
  }
}
```

```bash
# 构建命令
npm run tauri build
# 输出：
#   src-tauri/target/release/bundle/nsis/Clipboard-Lite_x.x.x_x64-setup.exe
#   src-tauri/target/release/clipboard-lite.exe  # portable executable
```

#### 6.3 异常处理矩阵

| 异常场景 | 降级策略 | 用户提示 |
|----------|----------|----------|
| 剪贴板监听失败 | 重试 3 次后暂停 | 托盘气泡通知 |
| SQLite 损坏 | 备份后重建数据库 | 设置页警告 |
| 快捷键冲突 | 不注册，使用默认值 | 设置页红色提示 |
| 插件加载失败 | 跳过 Pro 功能 | 日志记录 |
| 粘贴模拟失败 | 仅写入剪贴板 | Toast 提示手动粘贴 |

#### 6.4 细节优化

- [ ] 图片缩略图异步生成，不阻塞主线程
- [ ] 搜索防抖 150ms
- [ ] 列表虚拟滚动（>100 条时启用）
- [ ] 启动时不加载全部历史，仅取最近 50 条

#### 验收标准

- 安装包 < 15 MB
- 启动后仅托盘图标，任务栏无窗口
- 便携版解压即用，数据写入 `%APPDATA%`
- 所有异常场景有用户可理解的提示

---

## 4. 架构设计

### 4.1 整体架构图

```
┌─────────────────────────────────────────────────────────┐
│                     Vue 3 Frontend                       │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌─────────┐ │
│  │MainPanel │  │Favorites │  │ Settings │  │ Pro UI  │ │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘  └────┬────┘ │
│       └─────────────┴─────────────┴──────────────┘      │
│                         Pinia Stores                     │
└─────────────────────────┬───────────────────────────────┘
                          │ Tauri IPC (invoke / emit)
┌─────────────────────────┴───────────────────────────────┐
│                     Rust Backend                         │
│  ┌────────────┐  ┌──────────┐  ┌──────────────────┐  │
│  │ clipboard  │  │ storage  │  │ hotkey           │  │
│  │ watcher    │──│ (SQLite) │  │ (global shortcut)│  │
│  └────────────┘  └──────────┘  └──────────────────┘  │
│  ┌────────────┐  ┌──────────┐  ┌──────────────────┐  │
│  │ license    │  │ plugin   │  │ pro-plugin.dll   │  │
│  │ (offline)  │──│ loader   │──│ (动态加载)        │  │
│  └────────────┘  └──────────┘  └──────────────────┘  │
└─────────────────────────────────────────────────────────┘
                          │
              ┌───────────┴───────────┐
              │   OS APIs             │
              │ Clipboard / Tray /    │
              │ Autostart / Input     │
              └───────────────────────┘
```

### 4.2 数据流

```
[OS Clipboard] ──poll──▶ [clipboard.rs]
                              │
                    filter / dedup / hash
                              │
                              ▼
                        [storage.rs] ──▶ SQLite
                              │
                         emit event
                              ▼
                     [Vue Pinia Store] ──▶ UI 更新

[User Ctrl+Shift+V] ──▶ [hotkey.rs] ──▶ show window
[User Enter]        ──▶ [paste_clip] ──▶ clipboard + SendInput
```

### 4.3 开源 / Pro 边界

```
开源仓库（MIT）                    闭源仓库（私有）
├── src/                          ├── pro-plugin/
├── src-tauri/                    │   ├── src/lib.rs
│   ├── clipboard.rs              │   └── 加密/导出/短语实现
│   ├── storage.rs                └── release/
│   ├── hotkey.rs                     └── pro-plugin.dll
│   ├── plugin_loader.rs  ◄──── 加载 ────┘
│   └── license.rs
└── plugins/  (空目录 + .gitkeep)
```

---

## 5. 轻量化策略

### 5.1 安装包体积 < 15 MB

| 策略 | 预期节省 | 实施方式 |
|------|----------|----------|
| Tauri vs Electron | ~120 MB | 使用系统 WebView2（Windows 10+ 内置） |
| UI 按需引入 | ~200 KB | `unplugin-vue-components` + Naive UI resolver |
| 无多余依赖 | ~500 KB | 禁止 axios、lodash 全量；手写工具函数 |
| Rust release 优化 | ~2 MB | `opt-level = "z"`, `lto = true`, `strip = true` |
| 无嵌入图片资源 | ~100 KB | SVG 图标内联，无大图 |
| UPX 压缩（可选） | ~30% | 仅 portable 版使用 |

**Cargo.toml 优化**：
```toml
[profile.release]
opt-level = "z"     # 体积优先
lto = true
codegen-units = 1
strip = true
panic = "abort"
```

**vite.config.ts 优化**：
```ts
build: {
  minify: 'esbuild',
  rollupOptions: {
    output: {
      manualChunks: undefined  // 单 chunk，减少 HTTP 请求（本地无影响但减小文件数）
    }
  }
}
```

### 5.2 内存占用 < 30 MB（空闲）

| 策略 | 说明 |
|------|------|
| 窗口按需创建 | 启动时不创建 WebView，首次呼出时初始化 |
| 图片懒加载 | 缩略图存磁盘路径，列表不加载原图 |
| 限制内存缓存 | 前端 Store 最多保留 100 条，其余按需 fetch |
| 监听线程轻量 | 单线程轮询，无 tokio runtime 全量引入 |
| 避免内存泄漏 | 组件 unmount 取消 event listener；Rust Arc 及时 drop |

**Rust 依赖精简**：
```toml
# 仅引入必要 features
tokio = { version = "1", features = ["rt", "time"] }  # 不要 "full"
serde = { version = "1", features = ["derive"] }
```

### 5.3 启动速度 < 800 ms

| 阶段 | 目标 | 策略 |
|------|------|------|
| 进程启动 | < 200 ms | 无 Pro 插件时不加载 libloading |
| DB 初始化 | < 100 ms | WAL 模式；启动时不 migrate 大表 |
| 监听启动 | < 100 ms | 异步 spawn，不阻塞窗口 |
| 前端首屏 | < 400 ms | 主面板懒加载；Vite 预构建 |
| 托盘显示 | < 800 ms | 总计 |

**启动流程优化**：
```rust
fn main() {
    tauri::Builder::default()
        .setup(|app| {
            // 1. 快速初始化 DB（仅 open + pragma）
            storage::init_db()?;
            // 2. 异步启动剪贴板监听
            std::thread::spawn(|| clipboard::start_watcher(app.handle()));
            // 3. 注册快捷键
            hotkey::register_default(app.handle())?;
            // 4. 显示托盘（不显示窗口）
            tray::setup(app.handle())?;
            Ok(())
        })
        .run(...)
}
```

---

## 6. 开发指南

### 6.1 环境准备

```bash
# 前置依赖
# - Node.js 20+
# - Rust 1.77+（rustup）
# - Visual Studio Build Tools（Windows C++ 桌面开发）
# - WebView2（Windows 10/11 通常已内置）

# 初始化
npm create tauri-app@latest clipboard-lite -- --template vue-ts
cd clipboard-lite

# 添加插件
npm run tauri add sql
npm run tauri add global-shortcut
npm run tauri add autostart

# 前端依赖
npm install naive-ui pinia vue-router
npm install -D unplugin-vue-components unplugin-auto-import
```

### 6.2 日常开发命令

```bash
npm run tauri dev          # 开发模式（热重载）
npm run tauri build        # 生产构建
npm run tauri build -- --debug  # 调试构建
cargo test --manifest-path src-tauri/Cargo.toml  # Rust 单元测试
```

### 6.3 关键开发模式

#### Tauri Command 定义（Rust → 前端）

```rust
// src-tauri/src/commands/clip.rs
#[tauri::command]
pub async fn get_clips(
    state: State<'_, AppState>,
    limit: Option<u32>,
    search: Option<String>,
) -> Result<Vec<Clip>, String> {
    storage::get_clips(&state.db, limit.unwrap_or(50), search).map_err(|e| e.to_string())
}
```

```ts
// src/stores/clipboard.ts
import { invoke } from '@tauri-apps/api/core'

const clips = await invoke<Clip[]>('get_clips', { limit: 50, search: query.value })
```

#### 事件监听（Rust → 前端推送）

```rust
app.emit("clip-added", &new_clip)?;
```

```ts
import { listen } from '@tauri-apps/api/event'
listen<Clip>('clip-added', (event) => {
  store.addClip(event.payload)
})
```

### 6.4 Windows 特有注意事项

| 问题 | 解决方案 |
|------|----------|
| 剪贴板图片格式 | 监听 `CF_DIB` / `CF_BITMAP`，转 PNG 存磁盘 |
| 文件路径 | 监听 `CF_HDROP` |
| 密码框检测 | `GetForegroundWindow` + 窗口类名 `Edit` + `ES_PASSWORD` 样式 |
| 粘贴到活动窗口 | `SendInput` 模拟 Ctrl+V，前先 `SetForegroundWindow` |
| 管理员权限程序 | 无法粘贴到 elevated 窗口（系统限制，需 UAC 同等级） |

---

## 7. 里程碑与时间线

| 里程碑 | 内容 | 预估工时 | 累计 |
|--------|------|----------|------|
| M1 | Step 1 骨架可运行 | 1 天 | 1 天 |
| M2 | Step 2 剪贴板 + DB 闭环 | 2–3 天 | 4 天 |
| M3 | Step 3 快捷键 + 主面板 MVP | 2–3 天 | 7 天 |
| M4 | Step 4 收藏 + 设置 | 1–2 天 | 9 天 |
| M5 | Step 5 Pro 框架 + 授权 | 2–3 天 | 12 天 |
| M6 | Step 6 托盘 + 打包 +  polish | 1–2 天 | 14 天 |
| **MVP 交付** | Windows 安装包 + 开源仓库 | — | **~2–3 周** |

---

## 8. 风险与应对

| 风险 | 影响 | 应对 |
|------|------|------|
| WebView2 未安装（Win7/8） | 无法运行 | MVP 仅支持 Win10+；README 注明 |
| 剪贴板轮询 CPU 占用 | 用户体验 | 自适应间隔：无变化时 500ms，有变化时 200ms |
| 粘贴到某些程序失败 | 核心功能 | 降级为仅写入剪贴板 + 提示 |
| 快捷键被其他软件占用 | 无法呼出 | 启动检测 + 设置页引导修改 |
| Pro 插件 ABI 兼容性 | 升级破坏 | 插件 API 版本号 + 主程序兼容性检查 |
| SQLite 并发写入 | 数据损坏 | WAL 模式 + 单写者队列 |

---

## 9. 开源交付清单

### 9.1 仓库需包含

- [ ] 完整源码（src/ + src-tauri/）
- [ ] README.md（中/英）：功能介绍、截图、构建说明
- [ ] LICENSE（MIT 或 Apache-2.0）
- [ ] CONTRIBUTING.md
- [ ] GitHub Actions CI：lint + build 检查
- [ ] `.gitignore`（排除 plugins/*.dll、target/、node_modules/）
- [ ] docs/ 使用文档

### 9.2 仓库不包含

- `plugins/pro-plugin.dll`（闭源）
- 激活码生成私钥
- 用户数据库文件

### 9.3 CI 配置概要

```yaml
# .github/workflows/build.yml
jobs:
  build-windows:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: npm ci
      - run: npm run tauri build
      - uses: actions/upload-artifact@v4
        with:
          name: windows-installer
          path: src-tauri/target/release/bundle/nsis/*.exe
```

---

## 10. 下一步行动

1. **立即执行 Step 1**：初始化项目骨架，确认 `tauri dev` 可运行
2. **并行准备**：设计 UI 线框图（主面板 + 设置页）
3. **技术验证**：Windows 剪贴板图片监听 POC（独立 Rust 小程序）

确认本计划后，可按 Step 1 → Step 6 顺序逐步交付可运行代码。
