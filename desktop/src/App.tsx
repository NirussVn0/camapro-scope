import { useState } from "react";
import { Panel } from "@/components/Panel";
import { StageFrame } from "@/components/StageFrame";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { useSession } from "@/hooks/useSession";
import { useVirtualOutput } from "@/hooks/useVirtualOutput";
import { usePreview } from "@/hooks/usePreview";

const statusVariant = {
  Disconnected: "outline",
  Ready: "secondary",
  Streaming: "default",
  Error: "destructive",
} as const;

const outputVariant = {
  Idle: "outline",
  Running: "default",
  Stopped: "secondary",
} as const;

export function App() {
  const [port, setPort] = useState(8100);
  const [token, setToken] = useState("");
  const { sessionState, generation, selectedMode, handleStart, handleStop, handleDisconnect, handleReconnect } =
    useSession();
  const { outputState, error: outputError, busy: outputBusy, toggle: toggleOutput } = useVirtualOutput();
  const { active: previewActive, frames: previewFrames, error: previewError, busy: previewBusy, start: startPreview, stop: stopPreview } = usePreview();
  const streaming = sessionState === "Streaming";

  return (
    <main className="mx-auto flex min-h-screen w-full max-w-5xl flex-col gap-6 p-8">
      <header className="flex items-center justify-between">
        <div>
          <h1 className="font-heading text-2xl font-semibold tracking-tight">Camapro Scope</h1>
          <p className="text-muted-foreground text-sm">
            Linux Native Preview (Wayland) · Gen {generation} · {selectedMode}
          </p>
        </div>
        <Badge variant={statusVariant[sessionState]}>{sessionState}</Badge>
      </header>

      {/* Invariant D06: native preview renders directly to the container surface; raw frames never pass through React */}
      <StageFrame streaming={previewActive} fps={`${previewFrames} khung`} />

      <Panel className="flex flex-wrap items-center gap-3 p-4">
        <Button onClick={handleStart} disabled={sessionState !== "Ready"}>
          Bắt đầu camera
        </Button>
        <Button onClick={handleStop} disabled={!streaming} variant="secondary">
          Dừng camera
        </Button>
        {sessionState === "Disconnected" ? (
          <Button onClick={handleReconnect} variant="outline">
            Kết nối lại điện thoại
          </Button>
        ) : (
          <Button
            onClick={() => {
              handleDisconnect();
              // One session authority: dropping the phone must drop the stream too.
              if (previewActive) void stopPreview();
            }}
            variant="outline"
          >
            Ngắt kết nối
          </Button>
        )}
      </Panel>

      {/* G3.2: virtual camera output (gst-launch → v4l2sink → /dev/video0) */}
      <Panel className="flex flex-wrap items-center gap-3 p-4">
        <Button onClick={toggleOutput} disabled={outputBusy} variant="outline">
          Output ảo: {outputState === "Running" ? "Tắt" : "Bật"}
        </Button>
        <Badge variant={outputVariant[outputState]}>{outputState}</Badge>
        {outputError ? (
          <p className="w-full text-destructive text-sm whitespace-pre-wrap">{outputError}</p>
        ) : null}
      </Panel>

      {/* T3: phone stream preview (MJPEG → gst waylandsink; token from phone UI) */}
      <Panel className="flex flex-wrap items-center gap-3 p-4">
        <input
          type="number"
          min={1}
          max={65535}
          value={port}
          onChange={(e) => setPort(Number(e.target.value) || 8100)}
          aria-label="Cổng stream"
          className="w-24 rounded-md border border-input bg-transparent px-2 py-1 text-sm"
        />
        <input
          type="text"
          value={token}
          onChange={(e) => setToken(e.target.value)}
          placeholder="Token từ điện thoại"
          aria-label="Token stream"
          className="w-48 rounded-md border border-input bg-transparent px-2 py-1 text-sm"
        />
        {previewActive ? (
          <Button onClick={stopPreview} disabled={previewBusy} variant="secondary">
            Dừng preview
          </Button>
        ) : (
          <Button onClick={() => void startPreview(port, token)} disabled={previewBusy || token.length === 0}>
            Xem stream
          </Button>
        )}
        <Badge variant={previewActive ? "default" : "outline"}>
          {previewActive ? `${previewFrames} khung` : "Tắt"}
        </Badge>
        {previewError ? (
          <p className="w-full text-destructive text-sm whitespace-pre-wrap">{previewError}</p>
        ) : null}
      </Panel>
    </main>
  );
}
