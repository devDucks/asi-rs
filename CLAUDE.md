# asi-rs — Agent Development Guide

## Project Overview

Rust drivers for ZWO ASI astronomy cameras and EFW (Electronic Filter Wheel) devices. The drivers expose device state and accept commands over MQTT.

**Workspace layout:**
```
asi-rs/
├── libasi-sys/          # Raw bindgen FFI bindings to libASICamera2.so / libEFWFilter.so
├── libasi/src/
│   ├── camera.rs        # Safe Rust wrappers for camera FFI
│   └── efw.rs           # Safe Rust wrappers for EFW FFI
├── src/
│   ├── lib.rs
│   └── bin/
│       ├── ccd/         # Camera daemon — COMPILES, WORKS, IS THE REFERENCE
│       │   ├── main.rs
│       │   └── ccd.rs
│       ├── efw/         # Filter wheel daemon — WIP, MUST BE REFACTORED (see below)
│       │   ├── main.rs
│       │   └── efw.rs
│       └── test/        # Small test binary
└── docs/
    ├── ASICamera2-software-kevelopment-kit.pdf
    └── camera-driver-development.md  # Full SDK reference for agents
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

## The EFW Binary — WIP: Must Be Refactored

> **The original approach for the EFW has been abandoned. The EFW binary must be rewritten to follow the CCD pattern.**

### What Is Wrong With the Current EFW Code

The current `src/bin/efw/` implementation uses a completely different and now-rejected architecture:

1. **gRPC transport** — uses `tonic` and `lightspeed_astro` protobuf service definitions. This is being dropped entirely in favour of MQTT.
2. **Trait soup** — `BaseAstroDevice`, `AsiEfw`, `FilterWheel` traits. These are over-engineered abstractions that should not be carried forward.
3. **`Vec<Property>` property model** — properties are stored in a runtime-indexed `Vec<Property>` (from `lightspeed_astro`). This is fragile (positional access like `self.properties.get_mut(1)`) and should be replaced with plain named struct fields.
4. **`lightspeed_astro` dependency** — this crate brings in gRPC types and should be removed from the EFW binary entirely.

### Target State After Refactor

The refactored EFW binary must mirror the CCD binary in structure:

- Drop `tonic`, `tonic-reflection`, `lightspeed_astro` imports from the EFW binary
- Drop all traits (`BaseAstroDevice`, `AsiEfw`, `FilterWheel`) — replace with a plain `EfwDevice` struct
- Hold device state in `Arc<RwLock<EfwDevice>>`
- Use MQTT (`rumqttc`) for all communication — same broker at `127.0.0.1:1883`
- Assign a `Uuid` per device at startup
- Publish device state as JSON (`serde::Serialize`) periodically
- Subscribe to MQTT topics for commands (e.g. set slot, calibrate, set unidirectional)
- Spawn a Tokio task for periodic state fetching (current slot, etc.)
- Handle ctrl-c to cleanly close EFW devices

### EFW MQTT Topics to Implement (matching CCD pattern)

| Direction | Topic | Payload |
|---|---|---|
| Publish (state) | `devices/{uuid}` | JSON-serialized `EfwDevice` |
| Subscribe | `devices/{uuid}/set_slot` | slot number as string/int |
| Subscribe | `devices/{uuid}/calibrate` | trigger calibration |
| Subscribe | `devices/{uuid}/update` | generic property update |

### EFW Device Capabilities (from `libasi/src/efw.rs`)

The EFW has these operations:
- `get_num_of_connected_devices()` — device discovery
- `get_efw_id(index, &mut id)` — get SDK device ID
- `open_efw(id)` / `close_efw(id)` — lifecycle
- `get_efw_property(id, &mut info)` — fills `EFWInfo` (name, slot count)
- `get_efw_position(id)` → `i32` — current slot
- `set_efw_position(id, position)` — move to slot
- `calibrate_wheel(id)` — blocking; poll `check_wheel_is_moving(id)` until done
- `is_unidirectional(id)` / `set_unidirection(id, flag)` — direction mode

---

## General Coding Conventions

- **Error handling**: `libasi` wrappers call `check_error_code()` which logs errors via `log::error!`. Do not panic on SDK errors — log and continue where possible.
- **Logging**: use `env_logger` with `LS_LOG_LEVEL` env var. Default level is `info`.
- **Async**: Tokio multi-thread runtime. Blocking SDK calls (exposures, calibration) must be wrapped in `task::spawn_blocking`.
- **Shared state**: always `Arc<RwLock<T>>`. Prefer short lock scopes — drop the lock before any `await` or blocking call.
- **Serde**: device structs derive `Serialize`. Fields that are internal/not useful to clients are `#[serde(skip)]`.

## Build

```sh
cargo build                  # debug
cargo build --release        # release
cargo build --bin asi_ccd    # CCD daemon only
cargo build --bin asi_efw    # EFW daemon only
```

The `libasi-sys` crate uses `bindgen` at build time — requires `libASICamera2.so` and `libEFWFilter.so` to be present (see `libasi-sys/build.rs`).

## SDK Reference

- `docs/camera-driver-development.md` — full ASICamera2 SDK reference (enums, structs, functions, call sequences)
- `docs/efw-driver-development.md` — full EFW SDK reference (enums, structs, functions, call sequences, Rust notes)
