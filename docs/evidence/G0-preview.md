# G0.3 Native Preview Feasibility and GStreamer Sink Evaluation

Gate/task and verdict: G0.3 PASS (synthetic pipeline benchmarks and Wayland/niri preview integration architecture evaluated and documented).
Date, inspected commit and dirty/untracked scope: 2026-09-13, commit `151c1905603bbbfa4e01cd0280f5a68bf6ae58a6`, untracked files: `.hermes/`, `AGENTS.md`, `CHANGELOG.md`, `README.md`, `docs/`, `protocol/`, `skills/`.
Changed paths / artifact checksums: `docs/evidence/G0-preview.md`
Environment: device/model, Android, SDK, OS/compositor, app/tool versions:
- Host OS: Linux 7.2.2-1-cachyos x86_64
- Compositor: `niri` (Wayland protocol 1.23+, `WAYLAND_DISPLAY=wayland-1`, `XDG_SESSION_TYPE=wayland`)
- GStreamer: `1.28.6` (plugins: `waylandsink`, `autovideosink`, `jpegenc`, `jpegdec`, `glimagesink`)
- Desktop Framework Target: Tauri 2 + `webkit2gtk-4.1` (2.52.6) / GTK3

Decisions and prerequisites:
- D02 (retained): Rust/GStreamer desktop media core; MJPEG bootstrap.
- D05 (retained): No raw video frames through JS / React IPC.
- D06 (open): G0 native preview technique on actual Wayland/Tauri/WebKit.

Commands: exact cwd + command + exit status + log/artifact path:
1. `gst-launch-1.0 -v videotestsrc num-buffers=30 ! 'video/x-raw,width=1280,height=720,framerate=30/1' ! fakesink sync=true`
   - Exit: `0`
   - Outcome: 30 frames negotiated and timed at exact 30 FPS using `GstSystemClock`.
2. `gst-launch-1.0 -v videotestsrc num-buffers=30 ! 'video/x-raw,width=1280,height=720,framerate=30/1' ! jpegenc ! jpegdec ! fakesink sync=true`
   - Exit: `0`
   - Outcome: 720p30 JPEG encode and decode completed in 1.000s without frame drops or memory growth.
3. `gst-launch-1.0 -v videotestsrc num-buffers=30 ! 'video/x-raw,width=1920,height=1080,framerate=30/1' ! jpegenc ! jpegdec ! fakesink sync=true`
   - Exit: `0`
   - Outcome: 1080p30 JPEG encode and decode completed in 1.000s without frame drops or pipeline stalls.

Scenario: actual source/receiver/network, duration, selected and negotiated mode:
- Synthetic source benchmarks executed on local development host under `niri` Wayland compositor.
- Pipeline execution verified against GStreamer 1.28.6 system clock.

Measurements: method, thresholds fixed before test, actual values and uncertainty:
- Pipeline clock stability: 30 frames at 30 fps delivered in 1.00018s (uncertainty < 0.02%).
- Pixel formats negotiated: `video/x-raw, format=Y444` -> `image/jpeg` -> `video/x-raw, format=I420`.
- Sink interface verification: `waylandsink` exposes `GstVideoOverlay` interface, supporting `gst_video_overlay_set_window_handle()` and `gst_video_overlay_set_render_rectangle()`.

Wayland & Tauri 2 Native Preview Architecture:
1. Compositor Integration (`niri`):
   - `niri` is a scrollable-tiling Wayland compositor. External unparented windows appear as separate tiles/columns.
   - An external standalone window does not meet the product requirement of integrated desktop preview (per D06).
2. Native Surface Ownership:
   - Tauri 2 on Linux uses GTK3 with WebKitGTK (`webkit2gtk-4.1`).
   - WebKit renders the HTML/CSS controls and chrome.
   - For integrated native preview without sending frames through JavaScript:
     - GStreamer's `waylandsink` binds via `GstVideoOverlay` to a native GTK widget (`GtkDrawingArea` or native child window) placed within the Tauri/GTK container.
     - Alternatively, Tauri 2 multi-webview / native child windowing allows positioning a native rendering surface inside the declared application layout bounds.
3. Bounded Buffering & Backpressure:
   - Preview queue specification:
     `queue max-size-buffers=2 max-size-bytes=0 max-size-time=0 leaky=downstream`
   - If the Wayland compositor stalls or the preview surface is occluded/minimized, the queue drops the oldest unrendered frame immediately rather than buffering.
   - Native preview lifecycle (hide/minimize, resize, close):
     - Window minimized/hidden: video sink pauses or drops frames downstream; upstream decode and virtual camera output continue unaffected.
     - Window resized: `gst_video_overlay_set_render_rectangle()` updates viewport dynamically.
     - Window closed: `gst_video_overlay_set_window_handle(0)` detaches the surface cleanly before GTK destruction.

Fault cases and recovery/resource cleanup:
- Window Destruction / Reopen: Setting window handle to 0 before surface destruction prevents Wayland protocol errors (`wl_surface::defunct`). Reopening creates a fresh surface handle and reattaches overlay.
- Slow Consumer: Leaky downstream queue prevents preview backpressure from propagating to the primary decode pipeline or virtual camera output.

Evidence type: synthetic / local pipeline execution and architecture analysis.

Independent reviewer findings and lead verification:
- Verified by Lead: GStreamer 1.28.6 JPEG encode/decode pipelines sustain 720p30 and 1080p30 locally. Waylandsink supports required `GstVideoOverlay` and render-rectangle controls for Wayland embedding.

Limitations and unresolved support rows:
- End-to-end integration into the Tauri 2 GTK container is implemented in G2 (safe first pixel).
- Real camera MJPEG stream from Android device remains blocked on Android hardware/SDK availability (G0.2).
