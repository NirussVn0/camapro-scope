---
name: media-platform
description: "Use when changing Camera2 capture, GStreamer pipelines, native preview or virtual cameras; require bounded buffers, single ownership and measured hardware proof."
version: 1.0.0
author: Camapro Scope contributors
metadata:
  hermes:
    tags: [camapro-scope, project-workflow]
    related_skills: []
---

# Media and platform work

## Overview
Read media ownership in `docs/ARCHITECTURE.md`, `docs/PLATFORMS.md`, the selected roadmap gate and G0 evidence if it exists. Do not assume JPEG FPS or native preview compatibility from library names.

## When to use
Camera/encoder changes, MJPEG/H.264 transport, GStreamer decode/fan-out, Tauri native surfaces, virtual output, media performance or OS integration.

## Procedure
1. State the exact source → queue → transport → decoder → consumers path and every owning thread/resource. Completion: CameraEngine owns Camera2; one desktop decode pipeline; React receives status only.
2. Inventory every buffer, including socket/parser/jitter/decoder and each sink. Define frame/byte/time limits and drop policy. Inter-frame codecs require keyframe-aware recovery. Completion: no hidden unbounded stage behind a two-frame queue claim.
3. Add failing fake-source/slow-consumer and malformed-input tests first. Prove stop/partial-start/error closes readers, images, surfaces, codec, sockets and sinks. Completion: repeated start/stop and stale callbacks cannot leak/restart capture.
4. Test native preview creation, resize, hide/close/reopen on the actual OS/compositor. No base64/JPEG/raw frame IPC through React as an expedient fallback. Completion: integrated native surface proven; external debug window labeled spike-only.
5. Implement OS-specific behavior behind adapters. Detect missing/busy virtual camera and explain remediation; never install/load kernel modules or change permissions implicitly. Completion: fake-port tests plus explicit real-device evidence when authorized.
6. Negotiate and verify resolution, delivered FPS, pixel format/color, aspect, rotation and independent preview mirror. Slow preview must not stall virtual output. Completion: OBS/browser show current frames, not merely a device entry.
7. Measure with the development evidence template: actual hardware, fixed thresholds, sample interval, duration, memory trend, frame age and latency method. Completion: measured result compared with predeclared goal; simulated data clearly separated.
8. Check teardown after permission loss, desktop death, network drop and sink failure; check distribution/runtime licenses before packaging. Document BLOCKED when hardware/permissions are absent, never synthesize footage or measurement output.

## Common pitfalls
- Camera resolution advertisement does not prove sustainable MJPEG FPS.
- Linux native preview success does not imply Windows or all Wayland support.
- Wall clocks on different devices cannot establish one-way latency without calibration.
- Arbitrarily dropping compressed H.264 frames can destroy decoder recovery.

## Verification checklist
- [ ] Bounded total pipeline and independent sink backpressure.
- [ ] Native surface/resource lifecycle and pixel semantics verified.
- [ ] Host mutations separately authorized.
- [ ] Real-device evidence distinct from synthetic/build tests.
