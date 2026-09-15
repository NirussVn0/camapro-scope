# Camapro Scope — UI/UX Design System

This file encodes the owner's two UI/UX briefs (desktop + mobile, supplied
2026-09-15). They are canon: implement to this, don't re-interpret. When code
disagrees with this file, code is wrong or this file needs an explicit edit.

Core idea, both surfaces:

> The camera is the product. The UI should disappear when it is not needed.

This is a live webcam utility (phone → webcam on Linux). It is NOT a
recording dashboard, media manager, editor, or surveillance UI. If an element
does not help the user connect, configure, inspect, or use the webcam —
remove it. The app must be usable within ~5 seconds of opening.

## 1. Product context

- Desktop: receives + displays the phone's live stream, controls the virtual
  camera output, configures the session. Main controller.
- Mobile: camera source. Minimal: connect, stream, choose what the phone shows
  while streaming. Zero-duplication of desktop controls.

## 2. Visual direction (both)

Modern minimal dark utility. Feel: minimal camera tool + modern web dashboard
+ subtle glass. Developer/creator-grade, high information density without
crowding.

- Black / near-black background, dark-navy secondary surfaces.
- Soft blue accent (#60a5fa / #3b82f6). Blue = the interaction color.
- Very subtle borders, minimal shadows, small radius (8–12px).
- Clean modern typography (Geist on desktop; system sans on mobile).
- No excessive decorative elements. NOT macOS/GNOME/Windows mimicry, NOT a
  landing page, NOT a SaaS analytics dashboard. No fake desktops, docks,
  wallpapers, browser chrome, hero sections, promo cards.

### Tokens — desktop (`src/styles.css`, `.dark` block)

| Token | Value (spec hex → oklch where used) | Role |
|---|---|---|
| --background | #070b11 | app root |
| --surface | #0d131d | panels, sheet |
| --surface-glass | rgba(16,24,38,.72) + blur(16px) + 1px rgba(255,255,255,.08) border | navbar, sidebars, popovers, floating controls ONLY |
| --text-primary | #f5f7fa | text |
| --text-secondary | #94a3b8 | labels, captions |
| --accent | #60a5fa | selection, focus, active indicator |
| --accent-strong | #3b82f6 | primary button |
| --success | #4ade80 | connected/streaming |
| --danger | #f87171 | errors, connection lost |

Glass rules: preview surface stays visually solid (never transparent glass).
Blue glow only around active/selected controls. No purple neon.

## 3. Desktop layout (brief §3)

Three columns; navbar 56–64px above them.

```text
Logo  CamaPro Scope                                 ● Ready
──────────────────────────────────────────────────────────────
CAMERA           │      LIVE CAMERA FEED        │ DEVICE INFO
Front/Back       │      (65–75% of area)        │ Name/Res/FPS
ORIENTATION      │                              │ Format/Connection
Landscape/Portrait│                             │ NETWORK
IMAGE            │                              │ IP/Port/Status
Res/FPS/Aspect/  │                              │ [ Hide ]
Mirror           │                              │
```

- Left sidebar: camera configuration only, compact. Section headings
  (CAMERA / ORIENTATION / IMAGE), not giant cards per option. Segmented
  radios for camera & orientation (orientation items carry horizontal/vertical
  rect icons). Resolution (1920×1080 / 1280×720 / 854×480), FPS (24/30/60),
  aspect ratio (16:9/4:3/1:1), Mirror toggle — select controls, values not
  visible simultaneously.
- Center: preview dominates, ~16:9, nothing permanently overlaid. Small
  indicators allowed (top-right LIVE/res badge).
- Right sidebar: INFORMATION ONLY (Device: name/camera/resolution/fps/format/
  connection; Network: ip/port/status). Has a Hide button; when collapsed the
  preview expands into the freed space — never leave an empty column.
- Navbar: icon + name left, connection status right. Nothing else ever.

## 4. Desktop states

| State | Status pill | Preview |
|---|---|---|
| Disconnected | ● Not connected (gray) | pure black + app icon + "CamaPro Scope" + "Connect your phone to start" + "Waiting for camera..." + hint "Make sure your phone is connected" |
| Waiting/Ready | ● Waiting (blue) | as above |
| Connected/streaming | ● Connected (green) | live feed, no overlay panels |

Never demo footage / placeholder photography when offline. Connection state
understandable from the main interface — no intrusive dialogs.

## 5. Components (desktop)

- Buttons: compact, radius 8–12px, no giant CTAs. Selected/primary item =
  soft blue bg + thin blue border + small blue indicator; unselected =
  transparent/dark surface + subtle border.
- Icons: lucide only, one style.
- Live indicator: small dot + word, color = state semantic.

## 6. Responsive (desktop)

Min 1024px, ideal 1280–1920. Narrow → collapse right panel first, then shrink
left sidebar, preview keeps size priority.

## 7. Mobile spec (brief part 2)

Phone = camera source; near-zero interaction ideal:
Open → Connect → put phone somewhere → forget it; desktop controls everything.

- Main screen = camera screen. Disconnected: ENTIRE screen black (#000000),
  centered app icon + "CamaPro Scope" + "Not connected" (+ optional "Waiting
  for desktop"). Camera never auto-starts.
- Connected flow: Not connected → Connecting... → Connected → Camera streaming.
  Status dot colors: gray/blue/green/green; lost = red-orange "Connection
  lost" with "Reconnecting..." when auto-retry active. No modal dialogs.
- Display-while-connected preference (sheet): "Show Camera" | "Black Screen".
  Black Screen = camera still streaming to desktop, display black (privacy /
  distraction / power). MUST NOT change the outgoing stream.
  Model: Connected → Camera streams (always); display mode is orthogonal.
- Floating control: single small ••• button bottom-right (interferes least),
  NO permanent nav bar. Tap → compact glass bottom sheet (≤40–55% screen):
  CONNECTION (● status + desktop name, Connect/Disconnect), CAMERA (Front /
  Back radios — quick, ≤200ms fade, no flip animation), DISPLAY (2 radios),
  STREAMING (Run in Background toggle + caption "Keep streaming when the app
  is not in the foreground"). That's v1 — resolution/fps/orientation stay on
  desktop.
- Minimal UI mode (sheet closed): black or preview + one status dot top-left +
  the floating dot. "Camera is streaming even though the display is black"
  must be instantly obvious.
- Visual: bg #000000; glass panel rgba(15,20,30,.78) blur 20px border
  rgba(255,255,255,.08); text #F8FAFC / #94A3B8; accent #60A5FA; connected
  #4ADE80; error #F87171. Subtle gradients only. Large touch targets.
- Background promises must respect Android camera/foreground restrictions —
  never visually promise behavior the OS can't provide.

## 8. Forbidden (both), unless owner asks later

Recording, recording history, snapshot gallery, search, analytics/dashboard
stats, marketing cards, promo hero, desktop wallpaper/fake OS chrome, macOS
dock, fake browser frame, social/profile, unnecessary animations, giant
gradients, excessive neon, excessive glass panels, photo capture, filters,
beautification, bottom nav with multiple pages, fake shutter.

## 9. Language

UI labels follow the briefs' English strings (Ready / Connected / Streaming /
Not connected / Camera / Orientation / Image / Show Camera / Black Screen...).
The earlier Vietnamese labels are superseded by this spec; keep them only
where the briefs themselves show Vietnamese (none do).

## 10. Motion

Only functional: 150–200ms fades for camera switch / sheet. prefers-reduced-
motion respected (already global). One pulse on the LIVE dot max. No
decorative animation.

## 11. Deferred to real pipelines (G4), styled here as controls

Camera select, orientation, resolution/FPS/aspect, mirror, live device/network
telemetry are visually present per brief but inert until the WebSocket control
protocol ships (D06 stays: raw frames never through React). Controls must not
claim success they can't deliver: inert selects are allowed, fake results are
not. → wire when protocol commands land.

## 12. Accessibility floor

Visible labels for every input; focus-visible rings (accent); contrast: text
on bg ≥ 12:1, secondary ≥ 7:1, accent-on-dark ≥ 4.5:1; touch targets ≥ 48dp
mobile; status never conveyed by color alone (dot + word).
