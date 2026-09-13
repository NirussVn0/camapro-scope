# G0.1 Environment and Prerequisite Audit

Gate/task and verdict: G0.1 PASS (environment recorded); G0.2 BLOCKED (missing Android phone and SDK)
Date, inspected commit and dirty/untracked scope: 2026-09-13, commit `151c1905603bbbfa4e01cd0280f5a68bf6ae58a6`, untracked files: `.hermes/`, `AGENTS.md`, `CHANGELOG.md`, `README.md`, `docs/`, `protocol/`, `skills/`.
Changed paths / artifact checksums: `docs/evidence/G0-environment.md`
Environment: device/model, Android, SDK, OS/compositor, app/tool versions:
- Host OS: Linux 7.2.2-1-cachyos #1 SMP PREEMPT_DYNAMIC x86_64 (CachyOS)
- Compositor / Desktop: `niri` (Wayland, `XDG_SESSION_TYPE=wayland`, `WAYLAND_DISPLAY=wayland-1`, `DISPLAY=:1`)
- Python: 3.14.7 (`/usr/bin/python3`)
- Rust toolchain: `rustc 1.97.1 (8bab26f4f 2026-07-14)`, `cargo 1.97.1 (c980f4866 2026-06-30)`
- Node / Package manager: Node `v22.23.1`, pnpm `11.3.0`
- Java / JVM: OpenJDK `26.0.2` (default); OpenJDK `21` installed at `java-21-openjdk` (via `archlinux-java`)
- Gradle: Missing in PATH (`gradle: command not found`); no Gradle wrapper present in repository baseline
- Android SDK: Unset (`ANDROID_HOME` and `ANDROID_SDK_ROOT` are empty; no SDK directory found in standard paths `/opt`, `~/.android`, `~/Android`)
- ADB: Android Debug Bridge version `1.0.41` (Version `37.0.0-android-tools`, `/usr/bin/adb`)
- Connected Android hardware: `adb devices -l` reports 0 devices attached (`List of devices attached` is empty); `lsusb` reports no phone device connected; no emulator running
- GStreamer: `1.28.6` (`gst-launch-1.0 version 1.28.6`, pkg-config `gstreamer-1.0` version `1.28.6`)
- GStreamer plugins verified present: `v4l2sink`, `waylandsink`, `autovideosink`, `jpegdec`, `jpegenc`
- GTK & WebKit libraries: `webkit2gtk-4.1` (2.52.6), `gtk4` (4.22.4), `gtk+-3.0` (3.24.43)
- Virtual camera kernel driver: `v4l2loopback.ko.zst` (version 0.15.4) is installed in `/lib/modules/7.2.2-1-cachyos/updates/dkms/v4l2loopback.ko.zst`, but is currently NOT loaded (`lsmod | grep v4l2` is empty; no `/dev/video*` devices exist). Per repository invariants, module loading requires user authorization.
Decisions and prerequisites:
- D01 (retained): Linux desktop first. Verified that Linux environment has required GStreamer 1.28 and Wayland support.
- D06 (open): Wayland compositor is `niri`. Native preview spike must evaluate Wayland subsurface / window embedding on `niri`.
- D08 (open): Android reference phone and SDK floor missing; Linux reference distro is CachyOS (Arch-based) with `niri` Wayland compositor.
Commands: exact cwd + command + exit status + log/artifact path:
- `/home/nirussvn0/orca/projects/camapro-scope` | `uname -a` | exit 0
- `/home/nirussvn0/orca/projects/camapro-scope` | `python3 --version` | exit 0 -> `Python 3.14.7`
- `/home/nirussvn0/orca/projects/camapro-scope` | `rustc --version && cargo --version` | exit 0 -> `rustc 1.97.1`, `cargo 1.97.1`
- `/home/nirussvn0/orca/projects/camapro-scope` | `node --version && pnpm --version` | exit 0 -> `v22.23.1`, `11.3.0`
- `/home/nirussvn0/orca/projects/camapro-scope` | `archlinux-java status` | exit 0 -> `java-21-openjdk`, `java-26-openjdk (default)`
- `/home/nirussvn0/orca/projects/camapro-scope` | `adb version` | exit 0 -> `Android Debug Bridge version 1.0.41`
- `/home/nirussvn0/orca/projects/camapro-scope` | `adb devices -l` | exit 0 -> `List of devices attached` (empty)
- `/home/nirussvn0/orca/projects/camapro-scope` | `gst-launch-1.0 --version` | exit 0 -> `GStreamer 1.28.6`
- `/home/nirussvn0/orca/projects/camapro-scope` | `pkg-config --modversion gstreamer-1.0 webkit2gtk-4.1 gtk4` | exit 0 -> `1.28.6`, `2.52.6`, `4.22.4`
- `/home/nirussvn0/orca/projects/camapro-scope` | `modinfo v4l2loopback` | exit 0 -> version `0.15.4` present in DKMS updates
- `/home/nirussvn0/orca/projects/camapro-scope` | `lsmod | grep -E 'v4l2|videodev'` | exit 1 (no output)
- `/home/nirussvn0/orca/projects/camapro-scope` | `ls -l /dev/video*` | exit 2 -> `cannot access '/dev/video*': No such file or directory`
- `/home/nirussvn0/orca/projects/camapro-scope` | `python -m json.tool protocol/control-message.schema.json > /dev/null` | exit 0
Scenario: actual source/receiver/network, duration, selected and negotiated mode:
- Host environment inspection only. No streaming network scenario executed.
Measurements: method, thresholds fixed before test, actual values and uncertainty:
- Exact tool version probe and kernel module query.
Fault cases and recovery/resource cleanup:
- ADB daemon started automatically by probe; ADB server lifecycle confirmed. No persistent daemon or background service left running.
Evidence type: build / environment inspection (real host).
Independent reviewer findings and lead verification:
- Verified by Lead: all commands executed on local host; hardware absence confirmed by direct OS queries.
Limitations and unresolved support rows:
- G0.2 is BLOCKED until an Android reference device is connected and authorized over ADB, and an Android SDK / Gradle toolchain is configured (preferably using Java 21).
- G3 / virtual camera testing is blocked on loading `v4l2loopback`, which requires explicit user authorization.
- G0.3 (preview spike) can proceed on the Linux host using GStreamer + Wayland / niri.
