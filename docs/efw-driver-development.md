# ZWO EFW Filter Wheel SDK — Driver Development Reference

> Source: EFW Filter Wheel Software Development Kit, Revision 1.8.4 (2025-11-18)
> This document is structured for AI agents working on the `asi-rs` Rust EFW driver.

---

## Overview

The EFW SDK provides a C API for controlling ZWO Electronic Filter Wheel devices over USB. The Rust codebase wraps this via:
- `libasi-sys/` — raw bindgen FFI bindings to `libEFWFilter.so`
- `libasi/src/efw.rs` — safe Rust wrappers

Platform support: Windows, Linux (x86, x64, armv6, armv7, armv8), macOS (x64, arm64).
Library names: `libEFWFilter.so` (Linux), `libEFWFilter.dylib` (macOS), `EFW_filter.dll` (Windows).

> **Linux note:** If the SDK detects the device but cannot open it, install the udev rules:
> ```sh
> sudo cp efw.rules /etc/udev/rules.d/
> sudo udevadm control --reload-rules
> # then unplug and replug the device
> ```

---

## Structs & Enumerations

### `EFW_INFO`
Filled by `EFWGetProperty`. Contains the core device metadata.

```c
typedef struct _EFW_INFO {
    int  ID;          // Device ID — used in all subsequent API calls
    char Name[64];    // Human-readable device name
    int  slotNum;     // Number of filter slots on this wheel
} EFW_INFO;
```

### `EFW_ERROR_CODE`
Every API function returns one of these.

```c
EFW_SUCCESS              = 0   // operation successful
EFW_ERROR_INVALID_INDEX         // invalid index value
EFW_ERROR_INVALID_ID            // invalid ID value
EFW_ERROR_INVALID_VALUE         // invalid parameter
EFW_ERROR_REMOVED               // device not found, detected removal
EFW_ERROR_MOVING                // filter wheel is currently in motion
EFW_ERROR_ERROR_STATE           // filter wheel state error
EFW_ERROR_GENERAL_ERROR         // other error
EFW_ERROR_NOT_SUPPORTED         // device not supported
EFW_ERROR_INVALID_LENGTH        // data length not supported
EFW_ERROR_CLOSED                // device is closed
EFW_ERROR_END            = -1
```

### `EFW_ID`
```c
typedef struct _EFW_ID {
    unsigned char id[8];   // 8-byte alias for the device
} EFW_ID;
```

---

## API Functions

### Discovery

#### `EFWGetNum`
```c
int EFWGetNum()
```
Returns the number of connected EFW devices. **Must be the first function called.**

#### `EFWGetID`
```c
EFW_ERROR_CODE EFWGetID(int index, int *ID)
```
Retrieves the device ID for a given index (0-based, up to `EFWGetNum() - 1`).
- The returned `ID` is used in **all** subsequent API calls.
- The ID remains stable as long as the device is connected.

#### `EFWCheck`
```c
int EFWCheck(int iVID, int iPID)
```
Returns `1` if the device with the given VID/PID is an EFW. The EFW VID is `0x03C3`.
Replaces the deprecated `EFWGetProductIDs`.

#### `EFWGetProductIDs` *(deprecated)*
```c
int EFWGetProductIDs(int *pPIDs)
```
Deprecated. Use `EFWCheck` instead.

---

### Lifecycle

#### `EFWOpen`
```c
EFW_ERROR_CODE EFWOpen(int ID)
```
Opens the device with the given ID. Must return `EFW_SUCCESS` before any control operations are possible.

#### `EFWClose`
```c
EFW_ERROR_CODE EFWClose(int ID)
```
Closes the device and releases all resources. Must be the **last** call. After this, the device with this ID can no longer be controlled.

---

### Device Info

#### `EFWGetProperty`
```c
EFW_ERROR_CODE EFWGetProperty(int ID, EFW_INFO *pInfo)
```
Fills an `EFW_INFO` struct for the given device ID.
> **Must be called before `EFWSetPosition`**, because `EFWSetPosition` requires `slotNum` to validate the target position.

#### `EFWGetSDKVersion`
```c
char* EFWGetSDKVersion()
```
Returns the SDK version string.

#### `EFWGetFirmwareVersion`
```c
EFW_ERROR_CODE EFWGetFirmwareVersion(int ID, unsigned char *major, unsigned char *minor, unsigned char *build)
```
Retrieves firmware version for the specified device.

#### `EFWGetHWErrorCode`
```c
EFW_ERROR_CODE EFWGetHWErrorCode(int ID, int *pErrCode)
```
Retrieves the hardware-level firmware error code.

#### `EFWGetSerialNumber`
```c
EFW_ERROR_CODE EFWGetSerialNumber(int ID, EFW_SN *pSN)
```
Retrieves the serial number of the filter wheel.

---

### Position Control

#### `EFWGetPosition`
```c
EFW_ERROR_CODE EFWGetPosition(int ID, int *pPosition)
```
Retrieves the current filter slot position.
- Positions range from `0` to `slotNum - 1`.
- Returns `-1` while the wheel is rotating.

#### `EFWSetPosition`
```c
EFW_ERROR_CODE EFWSetPosition(int ID, int Position)
```
Commands the wheel to move to the specified slot.
- `Position` must be in `0..slotNum - 1`. Do not exceed `slotNum`.
- Returns `EFW_ERROR_MOVING` if the wheel is already in motion.
- `EFWGetProperty` must be called first to know `slotNum`.

---

### Direction Control

#### `EFWGetDirection`
```c
EFW_ERROR_CODE EFWGetDirection(int ID, bool *bUnidirectional)
```
Retrieves the current rotation direction setting.

#### `EFWSetDirection`
```c
EFW_ERROR_CODE EFWSetDirection(int ID, bool bUnidirectional)
```
Sets the rotation direction.
- `bUnidirectional = true`: wheel always rotates in the same direction.
- `bUnidirectional = false`: wheel takes the shortest path (bidirectional).

---

### Calibration

#### `EFWCalibrate`
```c
EFW_ERROR_CODE EFWCalibrate(int ID)
```
Enters calibration mode. This is a **blocking-style** operation from the SDK's perspective — the wheel rotates to find its home position. In the Rust driver, call this from `task::spawn_blocking` and poll `EFWGetPosition` (returns `-1` while moving) to detect completion.

---

### ID Management

#### `EFWSetID`
```c
EFW_ERROR_CODE EFWSetID(int ID, EFW_ID alias)
```
Sets an 8-byte alias for the device.

---

## Recommended Call Sequence

### 1. Establish Connection

```
EFWGetNum()
  → for each index 0..N-1:
      EFWGetID(index, &id)
EFWOpen(id)
EFWGetProperty(id, &info)   // fills name and slotNum — required before SetPosition
```

### 2. Communication / Control

| Goal | Function |
|---|---|
| Read current slot | `EFWGetPosition` |
| Move to slot | `EFWSetPosition` |
| Get/set direction | `EFWGetDirection` / `EFWSetDirection` |
| Calibrate | `EFWCalibrate` |
| Get device info | `EFWGetProperty` |
| Get firmware version | `EFWGetFirmwareVersion` |
| Get serial number | `EFWGetSerialNumber` |

### 3. Shutdown

```
EFWClose(id)   // call for every opened device
```

---

## Rust Binding Notes

The safe wrappers in `libasi/src/efw.rs` map to these SDK functions. Key points:

- All FFI calls are `unsafe`; the wrappers handle error logging.
- `EFWInfo` is a type alias for the bindgen-generated `_EFW_INFO`.
- **Calibration**: `libasi::efw::calibrate_wheel(id)` is a blocking FFI call — always run it inside `tokio::task::spawn_blocking`. Poll `libasi::efw::check_wheel_is_moving(id)` (which reads `EFWGetPosition` and checks for `-1`) until it returns `false`.
- **Position validity**: always read `EFW_INFO.slotNum` after `EFWGetProperty` and clamp/validate slot values before calling `EFWSetPosition`.

### Position While Moving

```rust
// EFWGetPosition returns -1 while the wheel is rotating
fn is_moving(id: i32) -> bool {
    libasi::efw::get_efw_position(id) == -1
}
```

---

## Key Gotchas for AI Agents

1. **`EFWGetID` index ≠ device ID**: The index is just an enumeration counter (`0..EFWGetNum()-1`). The actual `ID` returned by `EFWGetID` is what every other function takes. Always resolve index → ID before opening.

2. **`EFWGetProperty` before `EFWSetPosition`**: `SetPosition` requires knowing `slotNum`. Skipping `GetProperty` risks sending an out-of-range position.

3. **Position `-1` means moving**: `EFWGetPosition` returns `-1` while the wheel is in motion — not an error. Poll until it returns a valid slot index.

4. **`EFWCalibrate` is blocking at the hardware level**: The SDK call itself returns quickly, but the wheel takes time to complete. Must poll `EFWGetPosition` for `-1` to know when it finishes.

5. **`EFW_ERROR_MOVING`**: `EFWSetPosition` returns this if the wheel is already moving. The driver must check and wait or reject the command.

6. **`EFWClose` must be called for every opened device**: Failing to close leaks the USB handle. In the Rust driver, handle ctrl-c and call `close_efw` for each device.

7. **`EFWGetProductIDs` is deprecated**: Use `EFWCheck(0x03C3, pid)` instead.
