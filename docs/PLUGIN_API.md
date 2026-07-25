# Pro Plugin API

The Pro plugin API is intentionally small for the MVP. The host validates an offline license first, then loads a platform dynamic library from `plugins/`.

## Filenames

- Windows: `pro-plugin.dll`
- macOS: `libpro-plugin.dylib`
- Linux: `libpro-plugin.so`

The open-source repository keeps only `plugins/.gitkeep`; built Pro libraries should not be committed.

## ABI Version

Current host API version: `1`.

Plugins must export both symbols below.

```rust
#[repr(C)]
pub struct HostApi {
    pub api_version: u32,
}

#[repr(C)]
pub struct PluginInfo {
    pub api_version: u32,
    pub name: *const std::ffi::c_char,
    pub version: *const std::ffi::c_char,
    pub feature_flags: u64,
}

#[no_mangle]
pub unsafe extern "C" fn clipboard_lite_plugin_info() -> PluginInfo {
    // return static C strings owned by the plugin
}

#[no_mangle]
pub unsafe extern "C" fn clipboard_lite_plugin_load(host: *const HostApi) -> i32 {
    // return 0 on success
}
```

## Reserved Feature Flags

```text
bit 0: encrypted storage
bit 1: snippet templates
bit 2: import/export
bit 3: paste as plain text
```

The MVP only validates and loads the plugin. Menu/page registration is intentionally left for the next ABI revision so the open core remains stable.

## Offline Activation Code

Activation codes use:

```text
CLIP1.<base64url-json-payload>.<base64url-hmac-sha256-signature>
```

Payload shape:

```json
{
  "product": "clipboard-lite-pro",
  "seats": 2,
  "issuedAt": 1784870000000,
  "expiresAt": null,
  "deviceIds": ["*", "or-specific-device-fingerprint"],
  "features": ["encrypted-storage", "snippets", "import-export", "plain-text-paste"]
}
```

The local license file is encrypted with AES-256-GCM using a key derived from the current device fingerprint. A copied license file will not decrypt on another device.
