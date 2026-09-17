import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Panel } from "@/components/Panel";
import { StageFrame } from "@/components/StageFrame";
import { Button } from "@/components/ui/button";
import { useSession } from "@/hooks/useSession";
import { usePreview } from "@/hooks/usePreview";
import { useVirtualOutput } from "@/hooks/useVirtualOutput";
import { cn } from "@/lib/utils";
import logoUrl from "@/assets/logo.png";
import { generateQrDataUrl } from "@/lib/qr";
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
  const [host, setHost] = useState("127.0.0.1");
  const [port, setPort] = useState(8100);
  const [token, setToken] = useState("");
  const [qrDataUrl, setQrDataUrl] = useState<string | null>(null);
  const [qrModalOpen, setQrModalOpen] = useState(false);
  const [endpointHint, setEndpointHint] = useState<string | null>(null);
  const [pairingSuccess, setPairingSuccess] = useState<string | null>(null);
  const [checkingPhone, setCheckingPhone] = useState(false);
  const [phoneOnlineStatus, setPhoneOnlineStatus] = useState<string | null>(null);

  const [camera, setCamera] = useState("back");
  const [orientation, setOrientation] = useState("landscape");
  const [resolution, setResolution] = useState("1920x1080");
  const [fps, setFps] = useState("30");
  const [aspect, setAspect] = useState("16:9");
  const [mirror, setMirror] = useState(false);
  const [infoOpen, setInfoOpen] = useState(true);

  const { sessionState } = useSession();
  const { outputState, error: outputError, busy: outputBusy, toggle: toggleOutput } =
    useVirtualOutput();
  const {
    active: previewActive,
    frames: previewFrames,
    error: previewError,
    busy: previewBusy,
    start: startPreview,
    stop: stopPreview,
  } = usePreview();

  // Listen for real 2-way pairing events from desktop's pairing listener
  useEffect(() => {
    let unlistenFn: (() => void) | null = null;
    listen<{ host: string; port: number; token: string; name: string }>(
      "phone-paired",
      (event) => {
        const { host: phoneHost, port: phonePort, token: phoneToken, name: phoneName } = event.payload;
        setHost(phoneHost);
        setPort(phonePort);
        setToken(phoneToken);
        setPairingSuccess(`✓ Connected to ${phoneName || "Phone"} (${phoneHost})`);
        // Start streaming immediately on desktop
        void startPreview(phoneHost, phonePort, phoneToken);
        setTimeout(() => {
          setQrModalOpen(false);
          setPairingSuccess(null);
        }, 1800);
      },
    )
      .then((unlisten) => {
        unlistenFn = unlisten;
      })
      .catch((err) => console.error("Failed to listen for phone-paired:", err));

    return () => {
      if (unlistenFn) unlistenFn();
    };
  }, []);

  const status = previewActive
    ? { dot: "bg-[#4ade80]", text: "text-[#4ade80]", label: "Connected" }
    : sessionState === "Disconnected"
      ? { dot: "bg-[#94a3b8]", text: "text-[#94a3b8]", label: "Not connected" }
      : { dot: "bg-[#60a5fa]", text: "text-[#60a5fa]", label: "Waiting" };

  const handleParseEndpoint = (input: string) => {
    const trimmed = input.trim();
    if (!trimmed) return;
    setPhoneOnlineStatus(null);
    try {
      if (trimmed.startsWith("{") && trimmed.endsWith("}")) {
        const parsed = JSON.parse(trimmed);
        if (parsed.secret) setToken(parsed.secret);
        if (parsed.token) setToken(parsed.token);
        if (parsed.endpoint_hint) {
          const hint = parsed.endpoint_hint.replace(/^https?:\/\//, "").replace(/^ws:\/\//, "");
          const [h, p] = hint.split(":");
          if (h) setHost(h);
          if (p) setPort(Number(p) || 8100);
        } else if (parsed.ip) {
          setHost(parsed.ip);
          if (parsed.port) setPort(Number(parsed.port) || 8100);
        }
        return;
      }
      const url = new URL(trimmed.startsWith("http") ? trimmed : `http://${trimmed}`);
      if (url.hostname) setHost(url.hostname);
      if (url.port) setPort(Number(url.port));
      const t = url.searchParams.get("token");
      if (t) setToken(t);
    } catch {
      const parts = trimmed.split(":");
      if (parts.length === 2) {
        setHost(parts[0]);
        setPort(Number(parts[1]) || 8100);
      }
    }
  };

  const openQrPairing = async () => {
    try {
      setPairingSuccess(null);
      const p = (await invoke("generate_pairing_qr")) as Record<string, any>;
      const payloadStr = typeof p === "string" ? p : JSON.stringify(p);
      setEndpointHint(p.endpoint_hint || null);
      const dataUrl = await generateQrDataUrl(payloadStr, 220);
      setQrDataUrl(dataUrl);
      setQrModalOpen(true);
    } catch (e) {
      console.error("Failed to generate pairing QR:", e);
    }
  };

  const testPhoneReachability = async () => {
    setCheckingPhone(true);
    setPhoneOnlineStatus(null);
    try {
      const res = await invoke<{ online: boolean }>("check_phone_status", { host, port });
      if (res.online) {
        setPhoneOnlineStatus("✓ Phone is online");
      }
    } catch (e) {
      setPhoneOnlineStatus(`❌ ${String(e)}`);
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

        {/* Output */}
        <Panel className="flex flex-col gap-2 p-2.5">
          <Label>Output</Label>
          <Button
            onClick={toggleOutput}
            disabled={outputBusy}
            size="sm"
            variant={outputState === "Running" ? "default" : "outline"}
            className={cn("w-full gap-1.5 text-xs", outputState === "Running" && "bg-[#4ade80]/15 border-[#4ade80]/40 text-[#4ade80] hover:bg-[#4ade80]/20")}
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
                disabled={checkingPhone}
                className="text-[10px] text-[#94a3b8] hover:text-[#f5f7fa] transition-colors"
                title="Ping phone to check reachability"
              >
                {checkingPhone ? "Testing..." : "Test"}
              </button>
              <button
                onClick={async () => {
                  try {
                    const text = await navigator.clipboard.readText();
                    if (text) handleParseEndpoint(text);
                  } catch {
                    /* clipboard permission */
                  }
                }}
                className="text-[10px] text-[#60a5fa] hover:underline"
                title="Paste stream URL or IP:port from phone"
              >
                Paste URL
              </button>
            </div>
          </div>
          <div className="flex flex-col gap-1.5">
            <div className="flex gap-1.5">
              <input
                value={host}
                onChange={(e) => {
                  setHost(e.target.value);
                  setPhoneOnlineStatus(null);
                }}
                placeholder="127.0.0.1 or LAN IP"
                aria-label="Stream host"
                className="min-w-0 flex-1 rounded-lg border border-white/10 bg-[#0d131d] px-2 py-1 text-xs text-[#f5f7fa] outline-none focus-visible:ring-2 focus-visible:ring-[#60a5fa]/50"
              />
              <input
                type="number"
                min={1}
                max={65535}
                value={port}
                onChange={(e) => {
                  setPort(Number(e.target.value) || 8100);
                  setPhoneOnlineStatus(null);
                }}
                aria-label="Stream port"
                className="w-16 rounded-lg border border-white/10 bg-[#0d131d] px-2 py-1 text-xs text-[#f5f7fa] outline-none focus-visible:ring-2 focus-visible:ring-[#60a5fa]/50"
              />
            </div>
            <input
              value={token}
              onChange={(e) => setToken(e.target.value)}
              placeholder="Stream Token"
              aria-label="Stream token"
              className="w-full rounded-lg border border-white/10 bg-[#0d131d] px-2 py-1 font-mono text-xs text-[#f5f7fa] outline-none focus-visible:ring-2 focus-visible:ring-[#60a5fa]/50"
            />
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
              disabled={previewBusy || token.length === 0}
              size="sm"
              className="w-full bg-[#3b82f6] text-white hover:bg-[#60a5fa] text-xs"
            >
              Connect
            </Button>
          )}
          {previewError ? <p className="text-[#f87171] text-xs whitespace-pre-wrap">{previewError}</p> : null}
          <Button
            onClick={openQrPairing}
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
          className="fixed inset-0 z-50 flex items-center justify-center bg-black/75 p-4 backdrop-blur-sm animate-in fade-in duration-150"
        >
          <div className="relative flex w-full max-w-sm flex-col items-center gap-4 rounded-xl border border-white/10 bg-[#0d131d] p-5 shadow-2xl">
            <div className="flex w-full items-center justify-between">
              <div className="flex items-center gap-2">
                <QrCodeIcon className="size-4 text-[#60a5fa]" />
                <h3 className="text-sm font-semibold text-white">Desktop Pairing QR</h3>
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
                <p className="text-xs text-[#94a3b8]">Live video stream starting on PC...</p>
              </div>
            ) : (
              <>
                <div className="rounded-xl bg-white p-3 shadow-inner">
                  <img src={qrDataUrl} alt="Pairing QR Code" className="size-48 object-contain" />
                </div>

                <div className="flex flex-col items-center gap-1.5 text-center">
                  <div className="flex items-center gap-1.5 text-xs text-[#60a5fa]">
                    <span className="size-2 animate-pulse rounded-full bg-[#60a5fa]" />
                    <span className="font-medium">Waiting for phone to scan QR...</span>
                  </div>
                  {endpointHint ? (
                    <span className="rounded bg-white/5 px-2 py-0.5 font-mono text-[11px] text-[#94a3b8] select-all">
                      {endpointHint}
                    </span>
                  ) : null}
                  <p className="text-xs text-[#94a3b8] mt-1">
                    Open <span className="font-semibold text-[#f5f7fa]">Camapro Scope</span> on your phone and tap{" "}
                    <span className="font-semibold text-[#60a5fa]">Scan Desktop QR</span>. Both devices will pair automatically.
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
