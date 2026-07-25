# Contributing

Thanks for helping improve Clipboard Lite.

## Principles

- Keep the core local-first: no network requests, analytics, ads, or cloud services.
- Keep user data in the app data directory, never in the install directory.
- Keep Pro-only implementation out of the open-source core.
- Prefer small, focused changes with clear error handling.

## Development

```bash
npm install
npm run tauri dev
```

Before opening a pull request:

```bash
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
```

On Windows, Rust and Visual Studio Build Tools are required for Tauri builds.

## Pull Requests

- Describe the user-facing behavior changed.
- Include test notes.
- Do not commit `node_modules/`, `dist/`, `src-tauri/target/`, user databases, license files, or Pro plugin binaries.
