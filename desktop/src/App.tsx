import { Panel } from "@/components/Panel";
import { StageFrame } from "@/components/StageFrame";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { useSession } from "@/hooks/useSession";
import { useVirtualOutput } from "@/hooks/useVirtualOutput";

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
  const { sessionState, generation, selectedMode, handleStart, handleStop, handleDisconnect, handleReconnect } =
    useSession();
  const { outputState, error: outputError, busy: outputBusy, toggle: toggleOutput } = useVirtualOutput();
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
      <StageFrame streaming={streaming} />

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
          <Button onClick={handleDisconnect} variant="outline">
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
    </main>
  );
}
