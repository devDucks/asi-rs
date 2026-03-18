# CLAUDE.md

## Project Overview

`asi-rs` provides multiplatform native Rust drivers for ZWO ASI SDKs:
- ASI cameras (CCD)
- ASI filter wheels (EFW)
- ASI mounts
- ASI EAF (Electronic Automatic Focuser)

## Build

```bash
cargo build
cargo check
```

## Project Structure

- `src/bin/ccd/` - ASI CCD camera driver binary
- `src/bin/efw/` - ASI EFW filter wheel driver binary
- `src/bin/test/` - Test binary
- `libasi/` - Rust wrapper library for the ASI SDK
- `libasi-sys/` - Raw FFI bindings to the ASI C library
- `vendored/` - Vendored C libraries

## Key Dependencies

- `libasi` / `libasi-sys` - FFI bindings to ZWO ASI SDK
- `tokio` - Async runtime (multi-thread, signal, tracing)
- `rumqttc` - MQTT client for device communication
- `rfitsio` - FITS file I/O
- `serde` / `serde_json` - Serialization
- `log` / `env_logger` - Logging

## Development Notes

- Binaries communicate over MQTT
- Camera images are base64-encoded for transport
- Uses `uuid` for device identification
- `console-subscriber` is included for Tokio Console tracing support
