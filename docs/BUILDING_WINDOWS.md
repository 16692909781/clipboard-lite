# Building on Windows

Clipboard Lite needs the normal Tauri Windows toolchain.

## Required Tools

- Node.js 20+
- Rust stable with Cargo
- Visual Studio Build Tools with:
  - Desktop development with C++
  - MSVC toolchain
  - Windows 10/11 SDK
- WebView2 Runtime

## Install

Install Rust from:

```text
https://rustup.rs/
```

Install Visual Studio Build Tools from:

```text
https://aka.ms/vs/17/release/vs_BuildTools.exe
```

During Visual Studio Build Tools setup, select `Desktop development with C++`.

## Verify

```bash
rustc --version
cargo --version
npm.cmd run tauri info
```

## Run

```bash
npm.cmd install
npm.cmd run tauri dev
```

## Build

```bash
npm.cmd run tauri build
```

The NSIS installer is emitted under:

```text
src-tauri/target/release/bundle/nsis/
```

The portable executable is:

```text
src-tauri/target/release/clipboard-lite.exe
```
