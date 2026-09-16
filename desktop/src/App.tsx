import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Panel } from "@/components/Panel";
import { StageFrame } from "@/components/StageFrame";
import { Button } from "@/components/ui/button";
import { useSession } from "@/hooks/useSession";
import { usePreview } from "@/hooks/usePreview";
import { useVirtualOutput } from "@/hooks/useVirtualOutput";
import { cn } from "@/lib/utils";
import logoUrl from "@/assets/logo.png";
import { generateQrSvg } from "@/lib/qr";
import { Columns2Icon, PowerIcon, Rows3Icon, VideoIcon } from "lucide-react";

const selectCls =
  "w-full appearance-none rounded-lg border border-white/10 bg-[#0d131d] px-3 py-1.5 text-sm text-[#f5f7fa] outline-none focus-visible:ring-2 focus-visible:ring-[#60a5fa]/50";

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
            "flex flex-1 items-center justify-center gap-1.5 rounded-lg border px-2 py-1.5 text-xs transition-colors",
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
  const [port, setPort] = useState(8100);
  const [token, setToken] = useState("");
  const [qrPayload, setQrPayload] = useState<string | null>(null);
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

  const status = previewActive
    ? { dot: "bg-[#4ade80]", text: "text-[#4ade80]", label: "Connected" }
    : sessionState === "Disconnected"
      ? { dot: "bg-[#94a3b8]", text: "text-[#94a3b8]", label: "Not connected" }
      : { dot: "bg-[#60a5fa]", text: "text-[#60a5fa]", label: "Waiting" };

  return (
    <main className="flex h-screen flex-col overflow-hidden bg-[#070b11]">
      {/* Navbar — branding + status only */}
      <header className="flex h-14 shrink-0 items-center justify-between border-b border-white/5 bg-[#101826]/70 px-4 backdrop-blur-[16px]">
        <div className="flex items-center gap-2.5">
          <span className="flex size-7 items-center justify-center overflow-hidden rounded-lg border border-white/10">
            <img src={logoUrl} alt="" className="size-full object-cover" />
          </span>
          <span className="text-[15px] font-semibold tracking-tight">CamaPro Scope</span>
        </div>
        <span className={cn("flex items-center gap-1.5 text-xs font-medium", status.text)}>
          <span className={cn("size-1.5 rounded-full", status.dot, previewActive && "animate-pulse")} />
          {status.label}
        </span>
      </header>

      <div
        className={cn(
          "grid min-h-0 flex-1 gap-4 p-4",
          infoOpen ? "grid-cols-[220px_minmax(0,1fr)_240px]" : "grid-cols-[220px_minmax(0,1fr)]",
        )}
      >
        {/* Left — camera configuration */}
        <aside className="flex flex-col gap-4 overflow-y-auto">
          <Panel className="flex flex-col gap-2 p-3">
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
          <Panel className="flex flex-col gap-2 p-3">
            <Label>Orientation</Label>
            <Segmented
              value={orientation}
              onChange={setOrientation}
              options={[
                { value: "landscape", label: "Landscape", icon: <Columns2Icon className="size-3.5" /> },
                { value: "portrait", label: "Portrait", icon: <Rows3Icon className="size-3.5" /> },
              ]}
            />
          </Panel>
          <Panel className="flex flex-col gap-3 p-3">
            <Label>Image</Label>
            <div className="flex flex-col gap-1">
              <label className="text-[11px] text-[#94a3b8]" htmlFor="resolution">
                Resolution
              </label>
              <select id="resolution" className={selectCls} value={resolution} onChange={(e) => setResolution(e.target.value)}>
                <option>1920x1080</option>
                <option>1280x720</option>
                <option>854x480</option>
              </select>
            </div>
            <div className="flex flex-col gap-1">
              <label className="text-[11px] text-[#94a3b8]" htmlFor="fps">
                FPS
              </label>
              <select id="fps" className={selectCls} value={fps} onChange={(e) => setFps(e.target.value)}>
                <option>24</option>
                <option>30</option>
                <option>60</option>
              </select>
            </div>
            <div className="flex flex-col gap-1">
              <label className="text-[11px] text-[#94a3b8]" htmlFor="aspect">
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
              className="flex items-center justify-between rounded-lg border border-white/10 px-3 py-1.5 text-sm hover:bg-white/5"
            >
              <span className="text-[#f5f7fa]">Mirror</span>
              <span
                className={cn(
                  "relative h-4 w-7 rounded-full transition-colors",
                  mirror ? "bg-[#3b82f6]" : "bg-white/15",
                )}
              >
                <span
                  className={cn(
                    "absolute top-0.5 size-3 rounded-full bg-[#f5f7fa] transition-all",
                    mirror ? "left-3.5" : "left-0.5",
                  )}
                />
              </span>
            </button>
          </Panel>
          <Panel className="flex flex-col gap-2 p-3">
            <Label>Output</Label>
            <Button
              onClick={toggleOutput}
              disabled={outputBusy}
              size="sm"
              variant={outputState === "Running" ? "default" : "outline"}
              className={cn("w-full gap-1.5", outputState === "Running" && "bg-[#4ade80]/15 border-[#4ade80]/40 text-[#4ade80] hover:bg-[#4ade80]/20")}
            >
              <PowerIcon className="size-3.5" />
              Virtual camera: {outputState === "Running" ? "On" : "Off"}
            </Button>
            {outputError ? <p className="text-[#f87171] text-xs whitespace-pre-wrap">{outputError}</p> : null}
          </Panel>
          <Panel className="mt-auto flex flex-col gap-2 p-3">
            <Label>Connection</Label>
            <div className="flex gap-1.5">
              <input
                type="number"
                min={1}
                max={65535}
                value={port}
                onChange={(e) => setPort(Number(e.target.value) || 8100)}
                aria-label="Stream port"
                className="w-20 rounded-lg border border-white/10 bg-[#0d131d] px-2 py-1.5 text-sm outline-none focus-visible:ring-2 focus-visible:ring-[#60a5fa]/50"
              />
              <input
                value={token}
                onChange={(e) => setToken(e.target.value)}
                placeholder="Token"
                aria-label="Stream token"
                className="min-w-0 flex-1 rounded-lg border border-white/10 bg-[#0d131d] px-2 py-1.5 font-mono text-sm outline-none focus-visible:ring-2 focus-visible:ring-[#60a5fa]/50"
              />
            </div>
            {previewActive ? (
              <Button onClick={stopPreview} disabled={previewBusy} size="sm" variant="secondary" className="w-full gap-1.5">
                <VideoIcon className="size-3.5" /> Disconnect
              </Button>
            ) : (
              <Button
                onClick={() => void startPreview(port, token)}
                disabled={previewBusy || token.length === 0}
                size="sm"
                className="w-full bg-[#3b82f6] text-white hover:bg-[#60a5fa]"
              >
                Connect
              </Button>
            )}
            {previewError ? <p className="text-[#f87171] text-xs whitespace-pre-wrap">{previewError}</p> : null}
            <button
              onClick={async () => {
                try {
                  const p = await invoke("generate_pairing_qr");
                  setQrPayload(JSON.stringify(p, null, 2));
                } catch (e) {
                  setQrPayload(String(e));
                }
              }}
              className="w-full rounded-lg border border-white/10 px-2 py-1.5 text-xs text-[#94a3b8] hover:bg-white/5 hover:text-[#f5f7fa]"
            >
              Pair via QR (structural prep)
            </button>
            {qrPayload ? (
              <div className="flex flex-col items-center gap-1">
                <div
                  className="rounded-lg bg-white p-2"
                  dangerouslySetInnerHTML={{ __html: generateQrSvg(qrPayload, 160) }}
                />
                <p className="text-[10px] text-[#94a3b8]">Scan with phone camera</p>
              </div>
            ) : null}
          </Panel>
        </aside>

        {/* Center — the camera */}
        <section className="min-h-0">
          {/* D06: native waylandsink renders into #native-preview-surface-slot; React never touches frames */}
          <StageFrame streaming={previewActive} fps={`${resolution} · ${fps} FPS`} />
        </section>

        {/* Right — information only */}
        {infoOpen ? (
          <aside className="flex min-h-0 flex-col gap-4 overflow-y-auto">
            <Panel className="flex flex-col gap-2 p-3">
              <div className="flex items-center justify-between">
                <Label>Device</Label>
                <button
                  onClick={() => setInfoOpen(false)}
                  className="text-[11px] text-[#94a3b8] hover:text-[#f5f7fa]"
                >
                  Hide
                </button>
              </div>
              {(
                [
                  ["Name", previewActive ? "Android phone" : "—"],
                  ["Camera", `${camera[0].toUpperCase()}${camera.slice(1)} Camera`],
                  ["Resolution", previewActive ? resolution : "—"],
                  ["FPS", previewActive ? `${fps} FPS` : "—"],
                  ["Format", previewActive ? "MJPEG" : "—"],
                ] as const
              ).map(([k, v]) => (
                <div key={k} className="flex justify-between text-xs">
                  <span className="text-[#94a3b8]">{k}</span>
                  <span className="font-mono">{v}</span>
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
                  ["IP Address", previewActive ? "127.0.0.1" : "—"],
                  ["Port", previewActive ? String(port) : "—"],
                  ["Frames", String(previewFrames)],
                ] as const
              ).map(([k, v]) => (
                <div key={k} className="flex justify-between text-xs">
                  <span className="text-[#94a3b8]">{k}</span>
                  <span className="font-mono">{v}</span>
                </div>
              ))}
            </Panel>
          </aside>
        ) : null}
      </div>
    </main>
  );
}
