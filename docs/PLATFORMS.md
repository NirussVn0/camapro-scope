# Platform boundaries and support evidence

**No platform has been implemented or verified.** Linux-first is a delivery order, not a compatibility guarantee. Record actual OS/device versions and results before promoting any row to supported.

| Surface | Target | Required proof |
|---|---|---|
| Android sender | Kotlin/Camera2/foreground service | real-device permission/lifecycle, modes, cleanup, sustained stream |
| Linux desktop | Rust/GStreamer + Tauri controls | native preview on declared compositor, packaged runtime |
| Linux virtual camera | v4l2loopback / v4l2sink | device negotiation and frames in OBS + browser |
| Windows desktop/output | shared core + native adapter | supported Windows API/OS, media-source lifecycle, installer and two consumers |
| Wayland shortcuts | optional adapter; future CLI fallback | compositor-specific binding and single command authority |

## Android (G0–G4)

Freeze SDK/toolchain and at least one reference phone at G0; do not infer Camera2 support level or JPEG FPS from megapixel count. Test permission denied/revoked, notification stop, app background, screen-off, camera-in-use, service/process death and lens changes. Record manufacturer, model, Android version, camera IDs, advertised/actual resolution/FPS, thermal conditions and sensor-control support. Shared semantics must still handle empty camera lists and unsupported manual controls.

## Linux

Keep `core/` portable; native surface, virtual output, secure store, shortcuts and IPC implementations live under `desktop/src-tauri/src/platform/` when needed. OS selection belongs in the composition root, not scattered through business logic.

Virtual camera path: GStreamer → v4l2sink → existing v4l2loopback → `/dev/videoN`. Detect missing module, permission problems, busy device and unsupported formats. Report the exact remediation; never silently install/load a module, alter groups or elevate permissions. Setup is a separately user-approved host action, not part of app startup or routine tests.

G0 must select/prove native preview integration on the actual Tauri/WebKit/Wayland stack. Validate window/surface destruction, resize and hidden/minimized operation. An external GStreamer window is only a transport spike, not integrated preview completion.

Plan packaging after choosing one reference distribution/package format. Broad Arch/Ubuntu/Fedora/openSUSE support is unverified. Document GStreamer plugins, system/bundled ownership, native WebKit dependencies and upgrade behavior. Kernel modules cannot be assumed bundled into an AppImage.

Wayland global hotkeys are optional. The future CLI may be bound through compositor configuration, but is not implemented now. Its grammar must come from the same command contract; no separate parsing of exposure units or profile semantics in a shortcut callback.

## Windows (G0 feasibility; G8 delivery)

Media Foundation virtual camera is an architectural candidate, not an implementation or universal Windows compatibility claim. Before promising support, verify official API minimum OS/SDK, registration/media-source requirements, privacy/consent, frame negotiation, consumer compatibility and installer/uninstaller lifecycle. Record source URLs and version constraints in the D08 decision evidence. Do not invent Windows 10 fallback support; request a product decision if required.

Keep Windows code behind the platform port. Shared protocol/core tests must compile on Windows without Linux-only GStreamer/V4L2 dependencies. Do not run `--all-features` when platform features are mutually exclusive.

## Distribution gate

Inventory GStreamer/runtime and codec licenses and shipped plugins separately from application code. H.264 redistribution assumptions require review. Select one packaging format per proven platform first; additional installers follow measured demand. A clean-machine install/start/capture/stop/uninstall test is required. Windows support, signing and packaging are not validated by Linux CI.
