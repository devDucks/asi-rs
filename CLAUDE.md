# asi-rs — Agent Development Guide

## Project Overview

Rust drivers for ZWO ASI astronomy cameras and EFW (Electronic Filter Wheel) devices. The drivers expose device state and accept commands over MQTT.

**Workspace layout:**
```
asi-rs/
├── libasi-sys/          # Raw bindgen FFI bindings to libASICamera2.so / libEFWFilter.so
├── libasi/src/
│   ├── camera.rs        # Safe Rust wrappers for camera FFI
│   └── efw.rs           # Safe Rust wrappers for EFW FFI (complete)
├── src/
│   ├── lib.rs           # Shared utilities (asi_name_to_string, asi_id_to_string)
│   └── bin/
│       ├── ccd/         # Camera daemon — compiles, works, is the reference
│       │   ├── main.rs
│       │   └── ccd.rs
│       ├── efw/         # Filter wheel daemon — refactored, mirrors CCD pattern
│       │   ├── main.rs
│       │   └── efw.rs
│       └── test/        # Small test binary
├── vendored/
│   ├── camera/          # ASICamera2 libraries
│   └── efw/             # EFW libraries v1.8.4
│       ├── linux/{armv6,armv7,armv8,x64,x86}/
│       ├── mac/         # macOS x64
│       ├── mac_arm64/   # macOS Apple Silicon
│       └── windows/{x64,x86}/
└── docs/
    ├── ASICamera2-software-kevelopment-kit.pdf
    ├── EFW-SDK-quick-start-guide.pdf
    ├── camera-driver-development.md
    └── efw-driver-development.md
```

---

## The CCD Binary — Reference Implementation

The `ccd` binary is the **canonical pattern** for all device drivers in this project.

### Architecture

- Device state is held in `Arc<RwLock<AsiCamera>>`
- Communication protocol: **MQTT** via `rumqttc`
- Runtime: **Tokio** async
- Device state is serialized as JSON (`serde::Serialize`) and published periodically
- Each device gets a `Uuid` at startup

### MQTT Topics (CCD)

| Direction | Topic | Payload |
|---|---|---|
| Publish (state) | `devices/{uuid}` | JSON-serialized `AsiCamera` |
| Subscribe | `devices/{uuid}/expose` | triggers an exposure |
| Subscribe | `devices/{uuid}/update` | updates a property |

### Startup Sequence

```
AsiCcd::new()
  → look_for_devices()            // ASIGetNumOfConnectedCameras
  → for each index: AsiCamera::new(idx)
      → get_camera_info           // ASIGetCameraProperty
      → open_camera               // ASIOpenCamera
      → init_camera               // ASIInitCamera
      → get_num_of_controls
      → fetch_control_caps        // per-camera capabilities
      → get_camera_id             // reads/writes 8-byte ID to flash

Then in main():
  → subscribe to MQTT topics for each device
  → spawn ctrl-c handler (closes cameras cleanly)
  → spawn periodic state-publish task per device (every 2500ms)
  → MQTT event loop (poll for incoming expose/update messages)
```

### Key Types (CCD)

- `AsiCamera` — main device struct, `#[derive(Serialize)]`, owns all properties
- `AsiProperty` — internal struct describing a camera control cap (gain range, exposure range, etc.)
- `ROIFormat` (in libasi) — width/height/bin/img_type

### Platform Differences

Several FFI function signatures differ between Windows and Unix. The wrappers in `libasi/src/camera.rs` use `#[cfg(unix)]` / `#[cfg(windows)]` to handle this. Pay attention to:
- `get_control_value`: `value: &mut i64` on Unix, `&mut i32` on Windows
- `set_control_value`: `value: c_long` on Unix, `i32` on Windows
- `download_exposure`/`ASIGetDataAfterExp`: `buf_size: i64` on Unix, `i32` on Windows

---

## The EFW Binary

The EFW binary has been refactored and now mirrors the CCD pattern. Both binaries compile and follow the same architecture.

### Architecture

- Device state is held in `Arc<RwLock<EfwDevice>>`
- Communication protocol: **MQTT** via `rumqttc` (same broker at `127.0.0.1:1883`)
- Runtime: **Tokio** async
- Device state is serialized as JSON (`serde::Serialize`) and published periodically
- Each device gets a `Uuid` at startup

### MQTT Topics (EFW)

| Direction | Topic | Payload |
|---|---|---|
| Publish (state) | `devices/{uuid}` | JSON-serialized `EfwDevice` |
| Subscribe | `devices/{uuid}/set_slot` | slot number (1-indexed string) |
| Subscribe | `devices/{uuid}/calibrate` | triggers calibration |
| Subscribe | `devices/{uuid}/update` | generic property update (TODO) |

### Startup Sequence

```
AsiEfwDriver::new()
  → look_for_devices()            // EFWGetNum
  → for each index: EfwDevice::new(idx)
      → get_efw_id(index)         // EFWGetID — resolves index → ID
      → open_efw(id)              // EFWOpen
      → get_efw_property(id)      // fills name and slot_num
      → get_efw_position(id)      // initial slot
      → is_unidirectional(id)     // initial direction state

Then in main():
  → subscribe to MQTT topics for each device
  → spawn ctrl-c handler (closes devices cleanly)
  → spawn periodic state-publish task per device (every 2500ms)
  → MQTT event loop
```

### Key Types (EFW)

- `EfwDevice` — plain struct with named fields, `#[derive(Serialize)]`
  - `id: Uuid` — `#[serde(skip)]`
  - `name: String`
  - `efw_id: i32` — `#[serde(skip)]`, used for all SDK calls
  - `slot_num: i32` — total number of filter slots
  - `current_slot: i32` — 1-indexed; `0` means wheel is moving
  - `unidirectional: bool`
  - `calibrating: bool`

### Slot Indexing

The SDK uses 0-based slot positions. The `libasi::efw` wrappers convert to 1-based for all callers:
- `get_efw_position` adds 1 to the SDK result (returns `0` while moving)
- `set_efw_position` subtracts 1 before calling the SDK

### Calibration

Calibration is blocking at the hardware level. Always run it in `task::spawn_blocking`:

```rust
task::spawn_blocking(move || {
    let efw_id = device.read().unwrap().efw_id();
    device.write().unwrap().calibrating = true;
    libasi::efw::calibrate_wheel(efw_id);
    while libasi::efw::check_wheel_is_moving(efw_id) {
        std::thread::sleep(Duration::from_millis(100));
    }
    device.write().unwrap().calibrating = false;
});
```

### Full libasi EFW API

| Function | Description |
|---|---|
| `get_num_of_connected_devices()` | Device discovery — call first |
| `get_efw_id(index, &mut id)` | Resolve enumeration index → device ID |
| `open_efw(id)` / `close_efw(id)` | Lifecycle |
| `get_efw_property(id, &mut info)` | Fills `EFWInfo` (name, slot_num) |
| `get_efw_position(id)` | Current slot (1-indexed; 0 = moving) |
| `set_efw_position(id, pos)` | Move to slot (1-indexed) |
| `check_wheel_is_moving(id)` | Returns true while wheel rotates |
| `is_unidirectional(id)` / `set_unidirection(id, flag)` | Direction mode |
| `calibrate_wheel(id)` | Enter calibration mode |
| `get_sdk_version()` | SDK version string |
| `get_fw_error_code(id)` | Hardware-level firmware error code |
| `get_firmware_version(id)` | Returns `(major, minor, build)` |
| `get_serial_number(id)` | Returns `EFWId` (8-byte serial) |
| `set_id(id, alias)` | Write 8-byte alias to device flash |
| `get_product_ids()` | Deprecated; returns PID list |

---

## General Coding Conventions

- **Error handling**: `libasi` wrappers call `check_error_code()` which logs errors via `log::error!`. Do not panic on SDK errors — log and continue where possible.
- **Logging**: use `env_logger` with `LS_LOG_LEVEL` env var. Default level is `info`.
- **Async**: Tokio multi-thread runtime. Blocking SDK calls (exposures, calibration) must be wrapped in `task::spawn_blocking`.
- **Shared state**: always `Arc<RwLock<T>>`. Prefer short lock scopes — drop the lock before any `await` or blocking call.
- **Serde**: device structs derive `Serialize`. Fields that are internal/not useful to clients are `#[serde(skip)]`.
- **Topic parsing**: topics follow `devices/{UUID}/{action}`. UUID is 36 chars, so `&topic[8..44]` is the device ID and `&topic[45..]` is the action.

## Build

```sh
cargo build                  # debug
cargo build --release        # release
cargo build --bin asi_ccd    # CCD daemon only
cargo build --bin asi_efw    # EFW daemon only
```

The `libasi-sys` crate uses `bindgen` at build time — links against `vendored/efw/linux/x64/libEFWFilter.so` and `vendored/camera/linux/x64/libASICamera2.so` (paths hardcoded in `libasi-sys/build.rs`).

## Vendored Libraries

| Component | Version | Platforms |
|---|---|---|
| ASICamera2 | see `vendored/camera/` | linux x64/x86/armv6/7/8, mac, windows |
| EFWFilter | **1.8.4** | linux x64/x86/armv6/7/8, mac x64, mac arm64, windows x64/x86 |

## SDK Reference

- `docs/camera-driver-development.md` — full ASICamera2 SDK reference (enums, structs, functions, call sequences)
- `docs/efw-driver-development.md` — full EFW SDK reference (enums, structs, functions, call sequences, Rust notes)
