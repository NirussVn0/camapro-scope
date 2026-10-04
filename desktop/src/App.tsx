import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Panel } from "@/components/Panel";
import { StageFrame } from "@/components/StageFrame";
import { Button } from "@/components/ui/button";
import { usePreview } from "@/hooks/usePreview";
import { useVirtualOutput } from "@/hooks/useVirtualOutput";
import { cn } from "@/lib/utils";
import logoUrl from "@/assets/logo.png";
import { generateQrDataUrl } from "@/lib/qr";
import { canReplaceEndpoint, connectionLabel, connectionError, endpointError, parsePhoneEndpoint, type PhoneEndpoint } from "@/lib/connection";
import {
  Columns2Icon,
  PanelRightCloseIcon,
  PanelRightOpenIcon,
  PowerIcon,
  QrCodeIcon,
  Rows3Icon,
  VideoIcon,
  XIcon,
} from "lucide-react";

const selectCls =
  "w-full appearance-none rounded-lg border border-white/10 bg-[#0d131d] px-2.5 py-1.5 text-xs text-[#f5f7fa] outline-none focus-visible:ring-2 focus-visible:ring-[#60a5fa]/50";
const endpointLockedHint = "Disconnect local preview first (wait for Connect/Disconnect to finish) before pairing or changing phones. Use Stop on the phone to stop camera capture.";

function Segmented({
  options,
  value,
  onChange,
}: {
  options: { value: string; label: string; icon?: React.ReactNode }[];
  value: string;
  onChange: (v: string) => void;
}) {
  return (
    <div className="flex gap-1.5">
      {options.map((o) => (
        <button
          key={o.value}
          onClick={() => onChange(o.value)}
          aria-pressed={value === o.value}
          className={cn(
            "flex flex-1 items-center justify-center gap-1.5 rounded-lg border px-2 py-1 text-xs transition-colors",
            value === o.value
              ? "border-[#60a5fa]/40 bg-[#60a5fa]/15 text-[#60a5fa]"
              : "border-white/10 text-[#94a3b8] hover:bg-white/5",
          )}
        >
          {o.icon}
          {o.label}
        </button>
      ))}
    </div>
  );
}

function Label({ children }: { children: React.ReactNode }) {
  return <h3 className="text-[11px] font-medium tracking-wider text-[#94a3b8] uppercase">{children}</h3>;
}

export function App() {
  const [host, setHost] = useState("");
  const [port, setPort] = useState(8100);
  const [token, setToken] = useState("");
  const [qrDataUrl, setQrDataUrl] = useState<string | null>(null);
  const [qrModalOpen, setQrModalOpen] = useState(false);
  const [endpointHint, setEndpointHint] = useState<string | null>(null);
  const [pairingSuccess, setPairingSuccess] = useState<string | null>(null);
  const [pairedEndpoint, setPairedEndpoint] = useState<PhoneEndpoint | null>(null);
  const [checkingPhone, setCheckingPhone] = useState(false);
  const [phoneOnlineStatus, setPhoneOnlineStatus] = useState<string | null>(null);
  const [netDiag, setNetDiag] = useState<{
    vpn_active: boolean;
    vpn_service: string | null;
    lan_blocked_hint: string | null;
  } | null>(null);

  const [camera, setCamera] = useState("back");
  const [orientation, setOrientation] = useState("landscape");
  const [resolution, setResolution] = useState("1920x1080");
  const [fps, setFps] = useState("30");
  const [aspect, setAspect] = useState("16:9");
  const [mirror, setMirror] = useState(false);
  const [infoOpen, setInfoOpen] = useState(true);

  // Live Camera Controls (Gate G4)
  const [ev, setEv] = useState(0);
  const [iso, setIso] = useState("auto");
  const [shutter, setShutter] = useState("auto");
  const [focus, setFocus] = useState("auto");
  const [controlFeedback, setControlFeedback] = useState<string | null>(null);

  const applyCameraControls = async (changes: Record<string, any>) => {
    if (!token) return;
    try {
      await invoke("camera_set", {
        host,
        port,
        token,
        payload: {
          cameraId: camera === "back" ? "0" : "1",
          capabilityRevision: 1,
          changes,
        },
      });
      setControlFeedback("✓ Applied");
      setTimeout(() => setControlFeedback(null), 1500);
    } catch (e) {
      setControlFeedback(`❌ ${String(e)}`);
      setTimeout(() => setControlFeedback(null), 3000);
    }
  };

  const {
    outputState,
    devices: videoDevices,
    selectedDevice,
    setSelectedDevice,
    error: outputError,
    busy: outputBusy,
    toggle: toggleOutput,
  } = useVirtualOutput();

  const {
    active: previewActive,
    frames: previewFrames,
    error: previewError,
    busy: previewBusy,
    start: startPreview,
    stop: stopPreview,
    canEditEndpoint,
  } = usePreview();
  const endpointLocked = !canReplaceEndpoint({ active: previewActive, busy: previewBusy });

  // Network diagnostics on startup
  useEffect(() => {
    invoke<any>("check_network_environment")
      .then(setNetDiag)
      .catch((e) => console.error("Network check failed:", e));
  }, []);

  // Listen for real 2-way pairing events from desktop's pairing listener
  useEffect(() => {
    let unlistenFn: (() => void) | null = null;
    let disposed = false;
    listen<{ host: string; port: number; token: string; name: string }>(
      "phone-paired",
      (event) => {
        if (disposed) return;
        if (!canEditEndpoint()) {
          setPhoneOnlineStatus(`Pairing event ignored. ${endpointLockedHint}`);
          return;
        }
        const { host: phoneHost, port: phonePort, token: phoneToken, name: phoneName } = event.payload;
        setHost(phoneHost);
        setPort(phonePort);
        setToken(phoneToken);
        setPairedEndpoint({ host: phoneHost, port: phonePort, token: phoneToken });
        setPhoneOnlineStatus(null);
        setPairingSuccess(`✓ Paired / Ready: ${phoneName || "Phone"} (${phoneHost})`);
      },
    )
      .then((unlisten) => {
        if (disposed) unlisten();
        else unlistenFn = unlisten;
      })
      .catch((err) => console.error("Failed to listen for phone-paired:", err));

    return () => {
      disposed = true;
      if (unlistenFn) unlistenFn();
    };
  }, [canEditEndpoint]);

  const label = connectionLabel(previewActive, { host, port, token }, pairedEndpoint);
  const status = previewActive
    ? { dot: "bg-[#4ade80]", text: "text-[#4ade80]", label: "Connected" }
    : label === "Paired / Ready"
      ? { dot: "bg-[#60a5fa]", text: "text-[#60a5fa]", label }
      : { dot: "bg-[#94a3b8]", text: "text-[#94a3b8]", label };

  const handleParseEndpoint = (input: string) => {
    if (!canEditEndpoint()) {
      setPhoneOnlineStatus(endpointLockedHint);
      return;
    }
    setPhoneOnlineStatus(null);
    try {
      const endpoint = parsePhoneEndpoint(input);
      setHost(endpoint.host);
      setPort(endpoint.port);
      setToken(endpoint.token);
    } catch (e) {
      setPhoneOnlineStatus(e instanceof Error ? e.message : "Paste a valid phone URL or phone IP:port.");
    }
  };

  const openQrPairing = async () => {
    if (!canEditEndpoint()) {
      setPhoneOnlineStatus(endpointLockedHint);
      return;
    }
    try {
      setPairingSuccess(null);
      const p = (await invoke("generate_pairing_qr")) as Record<string, any>;
      const payloadStr = typeof p === "string" ? p : JSON.stringify(p);
      setEndpointHint(p.endpoint_hint || null);
      const dataUrl = await generateQrDataUrl(payloadStr, 220);
      if (!canEditEndpoint()) {
        setPhoneOnlineStatus(endpointLockedHint);
        return;
      }
      setQrDataUrl(dataUrl);
      setQrModalOpen(true);
    } catch (e) {
      console.error("Failed to generate pairing QR:", e);
      setPhoneOnlineStatus("Could not create pairing QR. Check desktop secure storage and network availability, then try again.");
    }
  };

  const testPhoneReachability = async () => {
    const invalid = endpointError(host, port);
    if (invalid) {
      setPhoneOnlineStatus(invalid);
      return;
    }
    setCheckingPhone(true);
    setPhoneOnlineStatus(null);
    try {
      const res = await invoke<{ online: boolean }>("check_phone_status", { host: host.trim(), port });
      setPhoneOnlineStatus(res.online ? "✓ Enrolled phone is reachable. Click Connect for preview." : "Phone is offline. Tap Start on the phone and check its endpoint.");
    } catch (e) {
      setPhoneOnlineStatus(`❌ ${connectionError(e)}`);
    } finally {
      setCheckingPhone(false);
    }
  };

  return (
    <main className="relative flex h-screen w-screen overflow-hidden bg-[#070b11] p-3 gap-3">
      {/* Left Sidebar — branding, camera controls, output, and connection */}
      <aside className="flex w-[230px] shrink-0 flex-col gap-3 overflow-y-auto pr-0.5">
        {/* Sleek inline header for tiling WMs: no wasted top bar */}
        <div className="flex items-center justify-between rounded-lg border border-white/5 bg-[#101826]/60 px-3 py-2 backdrop-blur-md">
          <div className="flex items-center gap-2">
            <span className="flex size-6 items-center justify-center overflow-hidden rounded-md border border-white/10 bg-white/5">
              <img src={logoUrl} alt="" className="size-full object-cover" />
            </span>
            <span className="text-xs font-semibold tracking-tight text-[#f5f7fa]">CamaPro Scope</span>
          </div>
          <span className={cn("flex items-center gap-1.5 text-[11px] font-medium", status.text)}>
            <span className={cn("size-1.5 rounded-full", status.dot, previewActive && "animate-pulse")} />
            {status.label}
          </span>
        </div>

        {/* Camera Selector */}
        <Panel className="flex flex-col gap-2 p-2.5">
          <Label>Camera</Label>
          <Segmented
            value={camera}
            onChange={setCamera}
            options={[
              { value: "front", label: "Front" },
              { value: "back", label: "Back" },
            ]}
          />
        </Panel>

        {/* Orientation */}
        <Panel className="flex flex-col gap-2 p-2.5">
          <Label>Orientation</Label>
          <Segmented
            value={orientation}
            onChange={setOrientation}
            options={[
              { value: "landscape", label: "Landscape", icon: <Columns2Icon className="size-3" /> },
              { value: "portrait", label: "Portrait", icon: <Rows3Icon className="size-3" /> },
            ]}
          />
        </Panel>

        {/* Image Configuration */}
        <Panel className="flex flex-col gap-2.5 p-2.5">
          <Label>Image</Label>
          <div className="flex flex-col gap-1">
            <label className="text-[10px] text-[#94a3b8]" htmlFor="resolution">
              Resolution
            </label>
            <select id="resolution" className={selectCls} value={resolution} onChange={(e) => setResolution(e.target.value)}>
              <option>1920x1080</option>
              <option>1280x720</option>
              <option>854x480</option>
            </select>
          </div>
          <div className="flex flex-col gap-1">
            <label className="text-[10px] text-[#94a3b8]" htmlFor="fps">
              FPS
            </label>
            <select id="fps" className={selectCls} value={fps} onChange={(e) => setFps(e.target.value)}>
              <option>24</option>
              <option>30</option>
              <option>60</option>
            </select>
          </div>
          <div className="flex flex-col gap-1">
            <label className="text-[10px] text-[#94a3b8]" htmlFor="aspect">
              Aspect Ratio
            </label>
            <select id="aspect" className={selectCls} value={aspect} onChange={(e) => setAspect(e.target.value)}>
              <option>16:9</option>
              <option>4:3</option>
              <option>1:1</option>
            </select>
          </div>
          <button
            onClick={() => setMirror(!mirror)}
            role="switch"
            aria-checked={mirror}
            className="flex items-center justify-between rounded-lg border border-white/10 px-2.5 py-1 text-xs hover:bg-white/5"
          >
            <span className="text-[#f5f7fa]">Mirror</span>
            <span
              className={cn(
                "relative h-3.5 w-6 rounded-full transition-colors",
                mirror ? "bg-[#3b82f6]" : "bg-white/15",
              )}
            >
              <span
                className={cn(
                  "absolute top-0.5 size-2.5 rounded-full bg-[#f5f7fa] transition-all",
                  mirror ? "left-3" : "left-0.5",
                )}
              />
            </span>
          </button>
        </Panel>

        {/* Live Camera Controls (Gate G4) */}
        <Panel className="flex flex-col gap-2 p-2.5">
          <div className="flex items-center justify-between">
            <Label>Controls</Label>
            {controlFeedback && (
              <span className="text-[10px] text-[#4ade80] font-mono">
                {controlFeedback}
              </span>
            )}
          </div>
          <div className="flex flex-col gap-1">
            <div className="flex justify-between text-[10px] text-[#94a3b8]">
              <span>Exposure (EV)</span>
              <span className="font-mono text-[#f5f7fa]">{ev > 0 ? `+${ev}` : ev}</span>
            </div>
            <input
              type="range"
              min={-4}
              max={4}
              step={1}
              value={ev}
              onChange={(e) => {
                const val = Number(e.target.value);
                setEv(val);
                void applyCameraControls({ exposureCompensationSteps: val });
              }}
              className="w-full accent-[#3b82f6] cursor-pointer"
            />
          </div>
          <div className="grid grid-cols-2 gap-1.5">
            <div className="flex flex-col gap-1">
              <label className="text-[10px] text-[#94a3b8]" htmlFor="ctrl-iso">
                ISO
              </label>
              <select
                id="ctrl-iso"
                className={selectCls}
                value={iso}
                onChange={(e) => {
                  const val = e.target.value;
                  setIso(val);
                  if (val === "auto") {
                    void applyCameraControls({ aeEnabled: true });
                  } else {
                    void applyCameraControls({ iso: Number(val), aeEnabled: false });
                  }
                }}
              >
                <option value="auto">Auto</option>
                <option value="100">100</option>
                <option value="200">200</option>
                <option value="400">400</option>
                <option value="800">800</option>
                <option value="1600">1600</option>
                <option value="3200">3200</option>
              </select>
            </div>
            <div className="flex flex-col gap-1">
              <label className="text-[10px] text-[#94a3b8]" htmlFor="ctrl-shutter">
                Shutter
              </label>
              <select
                id="ctrl-shutter"
                className={selectCls}
                value={shutter}
                onChange={(e) => {
                  const val = e.target.value;
                  setShutter(val);
                  if (val === "auto") {
                    void applyCameraControls({ aeEnabled: true });
                  } else {
                    void applyCameraControls({ shutterNanos: Number(val), aeEnabled: false });
                  }
                }}
              >
                <option value="auto">Auto</option>
                <option value="33333333">1/30s</option>
                <option value="16666666">1/60s</option>
                <option value="8333333">1/120s</option>
                <option value="4000000">1/250s</option>
                <option value="2000000">1/500s</option>
              </select>
            </div>
          </div>
          <div className="flex flex-col gap-1">
            <div className="flex justify-between text-[10px] text-[#94a3b8]">
              <span>Focus</span>
              <span className="font-mono text-[#f5f7fa]">{focus === "auto" ? "Auto" : `${focus} D`}</span>
            </div>
            <div className="flex items-center gap-2">
              <input
                type="range"
                min={0}
                max={10}
                step={0.5}
                disabled={focus === "auto"}
                value={focus === "auto" ? 0 : Number(focus)}
                onChange={(e) => {
                  const val = Number(e.target.value);
                  setFocus(String(val));
                  void applyCameraControls({ focusDistanceDiopters: val });
                }}
                className="w-full accent-[#3b82f6] cursor-pointer disabled:opacity-40"
              />
              <button
                type="button"
                onClick={() => {
                  if (focus === "auto") {
                    setFocus("0.0");
                    void applyCameraControls({ focusDistanceDiopters: 0.0 });
                  } else {
                    setFocus("auto");
                    void applyCameraControls({ aeEnabled: true });
                  }
                }}
                className={cn(
                  "rounded border px-1.5 py-0.5 text-[10px] transition-colors",
                  focus === "auto"
                    ? "border-[#60a5fa]/40 bg-[#60a5fa]/15 text-[#60a5fa]"
                    : "border-white/10 text-[#94a3b8] hover:bg-white/5",
                )}
              >
                Auto
              </button>
            </div>
          </div>
        </Panel>

        {/* Output Panel with Auto-detected Virtual Devices */}
        <Panel className="flex flex-col gap-2 p-2.5">
          <div className="flex items-center justify-between">
            <Label>Virtual Camera</Label>
            {videoDevices.length > 0 ? (
              <span className="text-[10px] text-[#4ade80] font-mono">
                {selectedDevice || videoDevices[0].path}
              </span>
            ) : null}
          </div>

          {videoDevices.length > 1 ? (
            <div className="flex flex-col gap-1">
              <label className="text-[10px] text-[#94a3b8]" htmlFor="vdev">
                Output Device
              </label>
              <select
                id="vdev"
                className={selectCls}
                value={selectedDevice}
                onChange={(e) => setSelectedDevice(e.target.value)}
              >
                {videoDevices.map((d) => (
                  <option key={d.path} value={d.path}>
                    {d.path} ({d.name})
                  </option>
                ))}
              </select>
            </div>
          ) : videoDevices.length === 1 ? (
            <div className="text-[10px] text-[#94a3b8] truncate">
              Device: <span className="font-mono text-[#f5f7fa]">{videoDevices[0].path}</span> ({videoDevices[0].name})
            </div>
          ) : null}

          <Button
            onClick={() => toggleOutput(selectedDevice)}
            disabled={outputBusy}
            size="sm"
            variant={outputState === "Running" ? "default" : "outline"}
            className={cn(
              "w-full gap-1.5 text-xs",
              outputState === "Running" &&
                "bg-[#4ade80]/15 border-[#4ade80]/40 text-[#4ade80] hover:bg-[#4ade80]/20",
            )}
          >
            <PowerIcon className="size-3.5" />
            Virtual camera: {outputState === "Running" ? "On" : "Off"}
          </Button>
          {outputError ? <p className="text-[#f87171] text-xs whitespace-pre-wrap">{outputError}</p> : null}
        </Panel>

        {/* Connection Controls */}
        <Panel className="mt-auto flex flex-col gap-2 p-2.5">
          <div className="flex items-center justify-between">
            <Label>Connection</Label>
            <div className="flex items-center gap-2">
              <button
                onClick={testPhoneReachability}
                disabled={checkingPhone || !!endpointError(host, port)}
                className="text-[10px] text-[#94a3b8] hover:text-[#f5f7fa] transition-colors"
                title="Check enrolled phone reachability"
              >
                {checkingPhone ? "Testing..." : "Test"}
              </button>
              <button
                onClick={async () => {
                  if (!canEditEndpoint()) return;
                  try {
                    const text = await navigator.clipboard.readText();
                    if (text) handleParseEndpoint(text);
                  } catch {
                    /* clipboard permission */
                  }
                }}
                disabled={endpointLocked}
                className="text-[10px] text-[#60a5fa] hover:underline"
                title="Paste stream URL or IP:port from phone"
              >
                Paste URL
              </button>
            </div>
          </div>

          {/* VPN Firewall notice if local LAN traffic is blocked by NordVPN */}
          {netDiag?.lan_blocked_hint && (
            <div className="rounded-lg border border-[#f59e0b]/40 bg-[#f59e0b]/10 p-2 text-[11px] text-[#f59e0b]">
              <div className="font-semibold flex items-center gap-1">
                <span>⚠️ VPN Blocking Local LAN</span>
              </div>
              <p className="mt-0.5 opacity-90 leading-tight">{netDiag.lan_blocked_hint}</p>
            </div>
          )}

          <div className="flex flex-col gap-1.5">
            <div className="flex gap-1.5">
              <input
                value={host}
                disabled={endpointLocked}
                onChange={(e) => {
                  if (!canEditEndpoint()) return;
                  setHost(e.target.value);
                  setPhoneOnlineStatus(null);
                }}
                placeholder="Phone IP (filled by QR pairing)"
                aria-label="Stream host"
                className="min-w-0 flex-1 rounded-lg border border-white/10 bg-[#0d131d] px-2 py-1 text-xs text-[#f5f7fa] outline-none focus-visible:ring-2 focus-visible:ring-[#60a5fa]/50"
              />
              <input
                type="number"
                min={1}
                max={65535}
                value={port}
                disabled={endpointLocked}
                onChange={(e) => {
                  if (!canEditEndpoint()) return;
                  setPort(Number(e.target.value));
                  setPhoneOnlineStatus(null);
                }}
                aria-label="Stream port"
                className="w-16 rounded-lg border border-white/10 bg-[#0d131d] px-2 py-1 text-xs text-[#f5f7fa] outline-none focus-visible:ring-2 focus-visible:ring-[#60a5fa]/50"
              />
            </div>
            <input
              value={token}
              disabled={endpointLocked}
              onChange={(e) => {
                if (!canEditEndpoint()) return;
                setToken(e.target.value);
                setPhoneOnlineStatus(null);
              }}
              placeholder="Stream Token"
              aria-label="Stream token"
              className="w-full rounded-lg border border-white/10 bg-[#0d131d] px-2 py-1 font-mono text-xs text-[#f5f7fa] outline-none focus-visible:ring-2 focus-visible:ring-[#60a5fa]/50"
            />
          </div>

          <p className="text-[11px] text-[#94a3b8]">
            LAN: Pair via QR first to enroll the phone and fill its endpoint, then click Connect. A pasted IP or token does not enroll a peer.
          </p>
          <p className="text-[11px] text-[#94a3b8]">
            {endpointLocked ? "Disconnect local preview before pairing or changing phones. " : ""}
            Phone Start starts camera capture. Desktop Disconnect stops only local preview; tap Stop on the phone to stop the camera.
          </p>
          {/* Quick connection shortcuts */}
          <div className="flex gap-1.5">
            <button
              onClick={() => {
                if (!canEditEndpoint()) return;
                setHost("127.0.0.1");
                setPort(8100);
                setToken("");
                setPhoneOnlineStatus("USB uses localhost only with ADB forwarding and enrollment for that endpoint. Prefer QR pairing over LAN.");
              }}
              disabled={endpointLocked}
              className="flex-1 rounded border border-white/10 px-1.5 py-1 text-[10px] text-[#94a3b8] hover:bg-white/5 hover:text-white transition-colors"
              title="USB requires 'adb forward tcp:8100 tcp:8100' and enrollment for the forwarded endpoint"
            >
              🔌 Use USB (ADB)
            </button>
          </div>

          {phoneOnlineStatus ? (
            <p className={cn("text-[11px]", phoneOnlineStatus.startsWith("✓") ? "text-[#4ade80]" : "text-[#f87171]")}>
              {phoneOnlineStatus}
            </p>
          ) : null}
          {previewActive ? (
            <Button onClick={stopPreview} disabled={previewBusy} size="sm" variant="secondary" className="w-full gap-1.5 text-xs">
              <VideoIcon className="size-3.5" /> Disconnect
            </Button>
          ) : (
            <Button
              onClick={() => void startPreview(host, port, token)}
              disabled={previewBusy || qrModalOpen || !!endpointError(host, port, token)}
              size="sm"
              className="w-full bg-[#3b82f6] text-white hover:bg-[#60a5fa] text-xs"
            >
              Connect
            </Button>
          )}
          {previewError ? <p className="text-[#f87171] text-xs whitespace-pre-wrap">{previewError}</p> : null}
          <Button
            onClick={openQrPairing}
            disabled={endpointLocked}
            size="sm"
            variant="outline"
            className="w-full gap-1.5 border-white/10 text-xs text-[#94a3b8] hover:bg-white/5 hover:text-[#f5f7fa]"
          >
            <QrCodeIcon className="size-3.5" /> Pair via QR
          </Button>
        </Panel>
      </aside>

      {/* Center — the video preview stage */}
      <section className="relative min-h-0 flex-1 overflow-hidden">
        {/* D06: native waylandsink renders into #native-preview-surface-slot */}
        <StageFrame streaming={previewActive} fps={`${resolution} · ${fps} FPS`} />

        {/* Restore Info panel button when collapsed (WM friendly) */}
        {!infoOpen && (
          <button
            onClick={() => setInfoOpen(true)}
            aria-label="Show info panel"
            className="absolute top-3 right-3 z-10 flex items-center gap-1.5 rounded-lg border border-white/10 bg-[#101826]/80 px-2 py-1 text-xs text-[#94a3b8] backdrop-blur-md hover:bg-white/10 hover:text-white transition-colors"
          >
            <PanelRightOpenIcon className="size-3.5" />
            <span>Info</span>
          </button>
        )}
      </section>

      {/* Right Sidebar — Info / Device / Network */}
      {infoOpen && (
        <aside className="flex w-[230px] shrink-0 flex-col gap-3 overflow-y-auto">
          <Panel className="flex flex-col gap-2 p-3">
            <div className="flex items-center justify-between">
              <Label>Device</Label>
              <button
                onClick={() => setInfoOpen(false)}
                aria-label="Hide info panel"
                className="flex items-center gap-1 text-[11px] text-[#94a3b8] hover:text-[#f5f7fa]"
              >
                <PanelRightCloseIcon className="size-3.5" />
                <span>Hide</span>
              </button>
            </div>
            {(
              [
                ["Name", previewActive ? "Android Phone" : "—"],
                ["Camera", `${camera[0].toUpperCase()}${camera.slice(1)} Camera`],
                ["Resolution", previewActive ? resolution : "—"],
                ["FPS", previewActive ? `${fps} FPS` : "—"],
                ["Format", previewActive ? "MJPEG" : "—"],
              ] as const
            ).map(([k, v]) => (
              <div key={k} className="flex justify-between text-xs">
                <span className="text-[#94a3b8]">{k}</span>
                <span className="font-mono text-[#f5f7fa]">{v}</span>
              </div>
            ))}
            <div className="flex justify-between text-xs">
              <span className="text-[#94a3b8]">Connection</span>
              <span className={cn("flex items-center gap-1", status.text)}>
                <span className={cn("size-1.5 rounded-full", status.dot)} /> {status.label}
              </span>
            </div>
          </Panel>

          <Panel className="flex flex-col gap-2 p-3">
            <Label>Network</Label>
            {(
              [
                ["Host", previewActive ? host : "—"],
                ["Port", previewActive ? String(port) : "—"],
                ["Frames", String(previewFrames)],
              ] as const
            ).map(([k, v]) => (
              <div key={k} className="flex justify-between text-xs">
                <span className="text-[#94a3b8]">{k}</span>
                <span className="font-mono text-[#f5f7fa]">{v}</span>
              </div>
            ))}
          </Panel>
        </aside>
      )}

      {/* QR Pairing Modal with Real-time 2-Way Confirmation */}
      {qrModalOpen && qrDataUrl ? (
        <div
          role="dialog"
          aria-modal="true"
          aria-labelledby="pairing-title"
          className="fixed inset-0 z-50 flex items-center justify-center bg-black/75 p-4 backdrop-blur-sm animate-in fade-in duration-150"
        >
          <div className="relative flex w-full max-w-sm flex-col items-center gap-4 rounded-xl border border-white/10 bg-[#0d131d] p-5 shadow-2xl">
            <div className="flex w-full items-center justify-between">
              <div className="flex items-center gap-2">
                <QrCodeIcon className="size-4 text-[#60a5fa]" />
                <h3 id="pairing-title" className="text-sm font-semibold text-white">Desktop Pairing QR</h3>
              </div>
              <button
                onClick={() => {
                  setQrModalOpen(false);
                  setPairingSuccess(null);
                }}
                aria-label="Close dialog"
                className="rounded-md p-1 text-[#94a3b8] hover:bg-white/10 hover:text-white"
              >
                <XIcon className="size-4" />
              </button>
            </div>

            {pairingSuccess ? (
              <div className="flex w-full flex-col items-center gap-2 rounded-xl bg-[#4ade80]/15 border border-[#4ade80]/30 p-6 text-center animate-in zoom-in-95">
                <span className="flex size-12 items-center justify-center rounded-full bg-[#4ade80]/20 text-2xl text-[#4ade80]">
                  ✓
                </span>
                <p className="text-sm font-semibold text-[#4ade80]">{pairingSuccess}</p>
                <p className="text-xs text-[#94a3b8]">Phone endpoint enrolled. Close this dialog and click Connect to start desktop preview.</p>
              </div>
            ) : (
              <>
                <div className="rounded-xl bg-white p-3 shadow-inner">
                  <img src={qrDataUrl} alt="Pairing QR Code" className="size-48 object-contain" />
                </div>

                <div className="flex flex-col items-center gap-1.5 text-center">
                  <div className="flex items-center gap-1.5 text-xs text-[#60a5fa]">
                    <span className="size-2 animate-pulse rounded-full bg-[#60a5fa]" />
                    <span className="font-medium">Waiting for phone Scan, then Start...</span>
                  </div>
                  {endpointHint ? (
                    <span className="rounded bg-white/5 px-2 py-0.5 font-mono text-[11px] text-[#94a3b8] select-all">
                      {endpointHint}
                    </span>
                  ) : null}
                  <p className="text-xs text-[#94a3b8] mt-1">
                    Open <span className="font-semibold text-[#f5f7fa]">Camapro Scope</span> on your phone and tap{" "}
                    <span className="font-semibold text-[#60a5fa]">Scan Desktop QR</span>, then tap{" "}
                    <span className="font-semibold text-[#60a5fa]">Start</span> on the phone. Start starts camera capture and completes the queued pairing. When Paired / Ready appears, close this dialog and click Connect for desktop preview. Desktop Disconnect stops only local preview; tap Stop on the phone to stop camera capture.
                  </p>
                </div>
              </>
            )}

            <Button
              onClick={() => {
                setQrModalOpen(false);
                setPairingSuccess(null);
              }}
              size="sm"
              variant="outline"
              className="w-full border-white/10 text-xs"
            >
              Close
            </Button>
          </div>
        </div>
      ) : null}
    </main>
  );
}
