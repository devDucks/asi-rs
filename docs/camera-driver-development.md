# ASICamera2 SDK — Camera Driver Development Reference

> Source: ZWO ASICamera2 Software Development Kit, Revision 2.9 (2023-01-11)
> This document is structured for AI agents working on the `asi-rs` Rust driver.

---

## Overview

The ASICamera2 SDK exposes a C API for controlling ZWO ASI cameras. The Rust codebase wraps this via:
- `libasi-sys/` — raw bindgen-generated FFI bindings
- `libasi/src/camera.rs` — safe Rust wrappers
- `src/bin/ccd/` — the main CCD daemon binary

Platform support: Windows (x86/x64), Linux, macOS. The library is `ASICamera2.so` on Linux.

---

## Enumerations

### `ASI_BAYER_PATTERN`
Bayer filter layout for color cameras.
```c
ASI_BAYER_RG = 0
ASI_BAYER_BG
ASI_BAYER_GR
ASI_BAYER_GB
```

### `ASI_IMG_TYPE`
Output image format.
```c
ASI_IMG_RAW8  = 0   // 8-bit grayscale, 1 byte/pixel
ASI_IMG_RGB24       // RGB color, 3 bytes/pixel (color cameras only)
ASI_IMG_RAW16       // 16-bit grayscale, 2 bytes/pixel, 65536 levels
ASI_IMG_Y8          // Monochrome 1 byte/pixel (color cameras only)
ASI_IMG_END = -1    // Sentinel — end of supported format list
```

### `ASI_GUIDE_DIRECTION`
ST4 guiding pulse direction.
```c
ASI_GUIDE_NORTH = 0
ASI_GUIDE_SOUTH
ASI_GUIDE_EAST
ASI_GUIDE_WEST
```

### `ASI_FLIP_STATUS`
```c
ASI_FLIP_NONE = 0   // no flip
ASI_FLIP_HORIZ      // horizontal flip
ASI_FLIP_VERT       // vertical flip
ASI_FLIP_BOTH       // horizontal + vertical
```

### `ASI_CAMERA_MODE`
Used with trigger cameras.
```c
ASI_MODE_NORMAL         = 0
ASI_MODE_TRIG_SOFT_EDGE
ASI_MODE_TRIG_RISE_EDGE
ASI_MODE_TRIG_FALL_EDGE
ASI_MODE_TRIG_SOFT_LEVEL
ASI_MODE_TRIG_HIGH_LEVEL
ASI_MODE_TRIG_LOW_LEVEL
ASI_MODE_END            = -1
```

### `ASI_ERROR_CODE`
Every API function returns one of these.
```c
ASI_SUCCESS                  = 0
ASI_ERROR_INVALID_INDEX         // no camera or index out of bounds
ASI_ERROR_INVALID_ID            // invalid camera ID
ASI_ERROR_INVALID_CONTROL_TYPE // invalid control type
ASI_ERROR_CAMERA_CLOSED         // camera not open
ASI_ERROR_CAMERA_REMOVED        // camera disconnected
ASI_ERROR_INVALID_PATH
ASI_ERROR_INVALID_FILEFORMAT
ASI_ERROR_INVALID_SIZE          // wrong video format size
ASI_ERROR_INVALID_IMGTYPE       // unsupported image format
ASI_ERROR_OUTOF_BOUNDARY        // startpos outside image boundary
ASI_ERROR_TIMEOUT
ASI_ERROR_INVALID_SEQUENCE      // must stop capture first
ASI_ERROR_BUFFER_TOO_SMALL
ASI_ERROR_VIDEO_MODE_ACTIVE
ASI_ERROR_EXPOSURE_IN_PROGRESS
ASI_ERROR_GENERAL_ERROR         // value out of valid range, etc.
ASI_ERROR_END
```

### `ASI_BOOL`
```c
ASI_FALSE = 0
ASI_TRUE
```

### `ASI_EXPOSURE_STATUS`
Used in snap-shot mode polling.
```c
ASI_EXP_IDLE    = 0   // ready to start
ASI_EXP_WORKING       // exposure in progress
ASI_EXP_SUCCESS       // complete, image ready to read
ASI_EXP_FAILED        // failed, restart required
```

---

## Structs

### `ASI_CAMERA_INFO`
Retrieved via `ASIGetCameraProperty`. Read-only camera metadata.

| Field | Type | Notes |
|---|---|---|
| `Name[64]` | `char[]` | Human-readable camera name |
| `CameraID` | `int` | Used in all subsequent API calls. Starts at 0. |
| `MaxHeight` | `long` | Max sensor height in pixels |
| `MaxWidth` | `long` | Max sensor width in pixels |
| `IsColorCam` | `ASI_BOOL` | Color vs mono |
| `BayerPattern` | `ASI_BAYER_PATTERN` | Only meaningful if `IsColorCam` |
| `SupportedBins[16]` | `int[]` | Supported bin values; 0-terminated. 1=bin1, 2=bin2, etc. |
| `SupportedVideoFormat[8]` | `ASI_IMG_TYPE[]` | `ASI_IMG_END`-terminated |
| `PixelSize` | `double` | Pixel size in µm |
| `MechanicalShutter` | `ASI_BOOL` | |
| `ST4Port` | `ASI_BOOL` | Has ST4 guide port |
| `IsCoolerCam` | `ASI_BOOL` | |
| `IsUSB3Host` | `ASI_BOOL` | |
| `IsUSB3Camera` | `ASI_BOOL` | |
| `ElecPerADU` | `float` | |
| `BitDepth` | `int` | Actual ADC bit depth |
| `IsTriggerCam` | `ASI_BOOL` | Supports trigger modes |

### `ASI_CONTROL_CAPS`
Describes the range and properties of a single control type. Retrieved per-camera via `ASIGetControlCaps`.

| Field | Type | Notes |
|---|---|---|
| `Name[64]` | `char[]` | e.g. `"Gain"`, `"Exposure"` |
| `Description[128]` | `char[]` | Human-readable description |
| `MaxValue` | `long` | |
| `MinValue` | `long` | |
| `DefaultValue` | `long` | |
| `IsAutoSupported` | `ASI_BOOL` | Can be auto-adjusted |
| `IsWritable` | `ASI_BOOL` | e.g. temperature is read-only |
| `ControlType` | `ASI_CONTROL_TYPE` | The enum ID |

> **Note:** `ASI_TEMPERATURE` min/max values are multiplied by 10 (e.g. 256 = 25.6°C).

### `ASI_CONTROL_TYPE`
All settable/readable camera parameters.

| Constant | Notes |
|---|---|
| `ASI_GAIN = 0` | |
| `ASI_EXPOSURE` | Microseconds |
| `ASI_GAMMA` | Range 1–100, nominally 50 |
| `ASI_WB_R` | Red white balance |
| `ASI_WB_B` | Blue white balance |
| `ASI_BRIGHTNESS` | Pixel value offset (bias, not scale) |
| `ASI_BANDWIDTHOVERLOAD` | Total data transfer rate % |
| `ASI_OVERCLOCK` | |
| `ASI_TEMPERATURE` | Sensor temp × 10 (read-only) |
| `ASI_FLIP` | Uses `ASI_FLIP_STATUS` values |
| `ASI_AUTO_MAX_GAIN` | Max gain during auto-adjust |
| `ASI_AUTO_MAX_EXP` | Max exposure during auto-adjust (µs) |
| `ASI_AUTO_MAX_BRIGHTNESS` | Target brightness during auto-adjust |
| `ASI_HARDWARE_BIN` | Hardware pixel binning |
| `ASI_HIGH_SPEED_MODE` | |
| `ASI_COOLER_POWER_PERC` | Cooler power % (cooled cameras only) |
| `ASI_TARGET_TEMP` | Target temp in °C — do NOT multiply by 10 |
| `ASI_COOLER_ON` | Open cooler |
| `ASI_MONO_BIN` | Smaller grid in software bin mode (color cameras) |
| `ASI_FAN_ON` | Fan control (cooled cameras only) |
| `ASI_PATTERN_ADJUST` | Only ASI1600 mono |
| `ASI_ANTI_DEW_HEATER` | |

### `ASI_ID`
```c
unsigned char id[8];  // 8-byte camera ID, stored in flash (USB3 only)
```

### `ASI_SUPPORTED_MODE`
```c
ASI_CAMERA_MODE SupportedCameraMode[16];  // ASI_MODE_END-terminated
```

---

## API Functions

### Camera Discovery & Lifecycle

#### `ASIGetNumOfConnectedCameras`
```c
int ASIGetNumOfConnectedCameras()
```
Returns count of connected ASI cameras. Call first.

#### `ASIGetCameraProperty`
```c
ASI_ERROR_CODE ASIGetCameraProperty(ASI_CAMERA_INFO *pASICameraInfo, int iCameraIndex)
```
Fills camera info struct for a given index (0-based). Can be called **before** opening the camera.

#### `ASIOpenCamera`
```c
ASI_ERROR_CODE ASIOpenCamera(int iCameraID)
```
Opens the camera. Must be called first before any camera operation. Does not affect other cameras.

#### `ASIInitCamera`
```c
ASI_ERROR_CODE ASIInitCamera(int iCameraID)
```
Initializes the camera. Must be called **after** `ASIOpenCamera`. Only affects the specified camera.

#### `ASICloseCamera`
```c
ASI_ERROR_CODE ASICloseCamera(int iCameraID)
```
Closes camera and releases all resources. Must be the last call.

---

### Controls

#### `ASIGetNumOfControls`
```c
ASI_ERROR_CODE ASIGetNumOfControls(int iCameraID, int *piNumberOfControls)
```

#### `ASIGetControlCaps`
```c
ASI_ERROR_CODE ASIGetControlCaps(int iCameraID, int iControlIndex, ASI_CONTROL_CAPS *pControlCaps)
```
`iControlIndex` is a 0-based index — **different from `ControlType`**.

#### `ASIGetControlValue`
```c
ASI_ERROR_CODE ASIGetControlValue(int iCameraID, ASI_CONTROL_TYPE ControlType, long *plValue, ASI_BOOL *pbAuto)
```

#### `ASISetControlValue`
```c
ASI_ERROR_CODE ASISetControlValue(int iCameraID, ASI_CONTROL_TYPE ControlType, long lValue, ASI_BOOL bAuto)
```
- When `bAuto=ASI_TRUE`, `lValue` should be the current value.
- Auto Exposure and Auto Gain **only work in video mode** (`ASIGetVideoData`), not snap mode.

---

### ROI & Image Format

#### `ASISetROIFormat`
```c
ASI_ERROR_CODE ASISetROIFormat(int iCameraID, int iWidth, int iHeight, int iBin, ASI_IMG_TYPE Img_type)
```
- `iWidth % 8 == 0`, `iHeight % 2 == 0` (general rule)
- USB2 camera ASI120: `iWidth * iHeight % 1024 == 0`
- `ASISetROIFormat` resets ROI start position to center. Call `ASISetStartPos` after to relocate.

#### `ASIGetROIFormat`
```c
ASI_ERROR_CODE ASIGetROIFormat(int iCameraID, int *piWidth, int *piHeight, int *piBin, ASI_IMG_TYPE *pImg_type)
```

#### `ASISetStartPos`
```c
ASI_ERROR_CODE ASISetStartPos(int iCameraID, int iStartX, int iStartY)
```
Position is relative to the **binned** image. Call after `ASISetROIFormat`.

#### `ASIGetStartPos`
```c
ASI_ERROR_CODE ASIGetStartPos(int iCameraID, int *piStartX, int *piStartY)
```

---

### Video (Continuous) Capture

#### `ASIStartVideoCapture`
```c
ASI_ERROR_CODE ASIStartVideoCapture(int iCameraID)
```

#### `ASIGetVideoData`
```c
ASI_ERROR_CODE ASIGetVideoData(int iCameraID, unsigned char *pBuffer, long lBuffSize, int iWaitms)
```
- Call repeatedly after `ASIStartVideoCapture`.
- Each call advances to the next frame; calling twice in rapid succession will not return the same frame.
- `iWaitms = -1` waits forever.
- Suggested timeout: `exposure_time * 2 + 500 ms`
- Buffer size requirements:
  - RAW8, Y8: `width * height`
  - RAW16: `width * height * 2`
  - RGB24: `width * height * 3`
- If read speed is insufficient, frames are discarded. Use a circular buffer with async processing.

#### `ASIStopVideoCapture`
```c
ASI_ERROR_CODE ASIStopVideoCapture(int iCameraID)
```

#### `ASIGetDroppedFrames`
```c
ASI_ERROR_CODE ASIGetDroppedFrames(int iCameraID, int *piDropFrames)
```

---

### Snap (Single Exposure) Mode

#### `ASIStartExposure`
```c
ASI_ERROR_CODE ASIStartExposure(int iCameraID)
```
Note: there is a setup time per snap shot; you cannot take two snapshots in shorter succession than this setup time.

#### `ASIGetExpStatus`
```c
ASI_ERROR_CODE ASIGetExpStatus(int iCameraID, ASI_EXPOSURE_STATUS *pExpStatus)
```
Poll continuously after `ASIStartExposure` until `ASI_EXP_SUCCESS` or `ASI_EXP_FAILED`.

#### `ASIStopExposure`
```c
ASI_ERROR_CODE ASIStopExposure(int iCameraID)
```
Cancels a long exposure early. If status was `ASI_EXP_SUCCESS` at stop time, image can still be read.

#### `ASIGetDataAfterExp`
```c
ASI_ERROR_CODE ASIGetDataAfterExp(int iCameraID, unsigned char *pBuffer, long lBuffSize)
```
Retrieve image after successful snap. Buffer sizing same as `ASIGetVideoData`.

---

### Dark Subtraction

#### `ASIEnableDarkSubtract`
```c
ASI_ERROR_CODE ASIEnableDarkSubtract(int iCameraID, char *pcBMPPath)
```
- `pcBMPPath`: path to a dark frame `.bmp` file (8-bit bitmap).
- The dark frame must be at `MaxWidth × MaxHeight` resolution.

#### `ASIDisableDarkSubtract`
```c
ASI_ERROR_CODE ASIDisableDarkSubtract(int iCameraID)
```

---

### Guiding (ST4)

Only cameras where `ASI_CAMERA_INFO.ST4Port == ASI_TRUE`.

#### `ASIPulseGuideOn`
```c
ASI_ERROR_CODE ASIPulseGuideOn(int iCameraID, ASI_GUIDE_DIRECTION direction)
```
Starts guide pulse. Must call `ASIPulseGuideOff` to stop.

#### `ASIPulseGuideOff`
```c
ASI_ERROR_CODE ASIPulseGuideOff(int iCameraID, ASI_GUIDE_DIRECTION direction)
```

---

### Camera Mode (Trigger Cameras)

Only relevant when `ASI_CAMERA_INFO.IsTriggerCam == ASI_TRUE`.

#### `ASIGetCameraSupportMode`
```c
ASI_ERROR_CODE ASIGetCameraSupportMode(int iCameraID, ASI_SUPPORTED_MODE *pSupportedMode)
```

#### `ASIGetCameraMode`
```c
ASI_ERROR_CODE ASIGetCameraMode(int iCameraID, ASI_CAMERA_MODE *mode)
```

#### `ASISetCameraMode`
```c
ASI_ERROR_CODE ASISetCameraMode(int iCameraID, ASI_CAMERA_MODE mode)
```

#### `ASISendSoftTrigger`
```c
ASI_ERROR_CODE ASISendSoftTrigger(int iCameraID, ASI_BOOL bStart)
```
- `bStart = ASI_TRUE`: camera starts exposing (edge trigger resets automatically when done).
- `bStart = ASI_FALSE`: stops exposure (required for level trigger).
- For edge triggers, no need to send `ASI_FALSE`.

---

### Utility

#### `ASIGetID` / `ASISetID`
```c
ASI_ERROR_CODE ASIGetID(int iCameraID, ASI_ID *pID)
ASI_ERROR_CODE ASISetID(int iCameraID, ASI_ID ID)
```
Read/write 8-byte ID to camera flash. **USB3 cameras only.**

#### `ASICameraCheck`
```c
ASI_BOOL ASICameraCheck(int iVID, int iPID)
```
Returns `ASI_TRUE` if the device with given VID/PID is an ASI camera.

#### `ASIGetProductIDs`
```c
int ASIGetProductIDs(int *pPIDs)
```
Deprecated — use `ASICameraCheck` instead.

#### `ASIGetSDKVersion`
```c
ASICAMERA_API char* ASIGetSDKVersion()
```
Returns SDK version string.

---

## Suggested Call Sequence

### 1. Initialization

```
ASIGetNumOfConnectedCameras()
  → for each index: ASIGetCameraProperty()   // can be done before open
ASIOpenCamera(cameraID)                       // first call to start camera
ASIInitCamera(cameraID)                       // second call to start camera
ASIGetNumOfControls(cameraID)
  → for each control: ASIGetControlCaps()
ASISetROIFormat(cameraID, width, height, bin, imgType)
ASISetStartPos(cameraID, startX, startY)      // only if not centered
```

### 2. Get / Set Controls

```
ASIGetControlValue(cameraID, controlType, &value, &isAuto)
ASISetControlValue(cameraID, controlType, value, isAuto)
// ASISetControlValue is allowed during capture except for exposure in trigger mode
```

### 3. Camera Mode (trigger cameras only)

```
// Check ASI_CAMERA_INFO.IsTriggerCam first
ASIGetCameraSupportMode(cameraID, &supportedMode)
ASISetCameraMode(cameraID, mode)
ASIGetCameraMode(cameraID, &mode)
```

### 4a. Capture — Video Mode

```
ASIStartVideoCapture(cameraID)

// In a single thread:
while capturing:
    if ASIGetVideoData(cameraID, buf, bufSize, waitMs) == ASI_SUCCESS:
        process(buf)

ASIStopVideoCapture(cameraID)
```

> Trigger mode cameras can **only** capture in video mode.

### 4b. Capture — Snap Shot Mode

```
ASIStartExposure(cameraID)

while true:
    ASIGetExpStatus(cameraID, &status)
    if status != ASI_EXP_WORKING: break

// Optional early cancel:
ASIStopExposure(cameraID)

if status == ASI_EXP_SUCCESS:
    ASIGetDataAfterExp(cameraID, buf, bufSize)
```

### 5. Close

```
ASICloseCamera(cameraID)   // call for every opened camera
```

---

## Rust Binding Notes

The Rust wrappers in `libasi/src/camera.rs` follow these conventions:

- All FFI calls are `unsafe`; the safe wrappers call `check_error_code()` which logs errors.
- Platform differences: on Unix `value` types are `i64`/`c_long`; on Windows they are `i32`.
- The `libasi-sys` crate uses `bindgen` to generate bindings from the C header at build time.
- `AsiCameraInfo`, `AsiControlCaps`, `AsiID` are type aliases to the raw bindgen structs.

### Buffer Size Formula (implement in Rust)

```rust
fn buffer_size(width: i32, height: i32, img_type: AsiImgType) -> usize {
    let pixels = (width * height) as usize;
    match img_type {
        AsiImgType::Raw8 | AsiImgType::Y8  => pixels,
        AsiImgType::Raw16                   => pixels * 2,
        AsiImgType::Rgb24                   => pixels * 3,
        _                                   => panic!("unknown img type"),
    }
}
```

### ROI Alignment Constraints

```rust
// General cameras
assert!(width % 8 == 0);
assert!(height % 2 == 0);

// USB2 ASI120 only
assert!((width * height) % 1024 == 0);
```

---

## Key Gotchas for AI Agents

1. **Open then Init**: `ASIOpenCamera` must precede `ASIInitCamera`. Skipping either returns `ASI_ERROR_CAMERA_CLOSED`.

2. **ROI resets start position**: After `ASISetROIFormat`, the start position moves to center. Always call `ASISetStartPos` after if a specific ROI offset is needed.

3. **Auto controls only work in video mode**: Setting `bAuto=ASI_TRUE` on gain/exposure has no effect in snap mode.

4. **Temperature is read-only and scaled**: `ASI_TEMPERATURE` values from `ASIGetControlValue` must be divided by 10 to get °C. `ASI_TARGET_TEMP` is NOT scaled.

5. **Trigger mode is video-only**: In any trigger mode, images can only be retrieved via `ASIGetVideoData`, not `ASIGetDataAfterExp`.

6. **`ASIGetVideoData` is frame-advancing**: Calling it twice fast skips a frame. Use a dedicated capture thread with a circular buffer.

7. **Buffer must be pre-allocated**: Always allocate buffer before starting capture using the formula above.

8. **`ASISetROIFormat` width/height constraints**: Violating alignment returns `ASI_ERROR_INVALID_SIZE`.

9. **`ASIGetControlCaps` index ≠ ControlType**: Iterate by index 0..N, read `ControlType` from the caps struct.

10. **`ASIStopExposure` can still yield an image**: If exposure was at `ASI_EXP_SUCCESS` when stopped, the image is still readable.
