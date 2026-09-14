import { Badge } from "@/components/ui/badge";
import { Empty, EmptyHeader, EmptyMedia, EmptyTitle, EmptyDescription } from "@/components/ui/empty";
import { MonitorIcon } from "lucide-react";

export function StageFrame({ streaming, fps }: { streaming: boolean; fps?: string }) {
  return (
    <div className="relative aspect-video w-full max-h-full max-w-full self-center justify-self-center overflow-hidden rounded-2xl border border-white/10 shadow-2xl shadow-black/50">
      {/* Invariant D06: native preview renders to this surface; never passing frames through React.
          The native GStreamer waylandsink window overlays this exact rect. Inner area stays transparent
          and free of React children so surface lifecycle attachment never conflicts with re-renders. */}
      <div id="native-preview-surface-slot" className="absolute inset-0 bg-transparent" />
      {!streaming && (
        <div className="absolute inset-0">
          <Empty className="h-full">
            <EmptyHeader>
              <EmptyMedia variant="icon">
                <MonitorIcon />
              </EmptyMedia>
              <EmptyTitle>Chưa có tín hiệu camera</EmptyTitle>
              <EmptyDescription>
                Kết nối điện thoại và bắt đầu phiên để xem video tại đây.
              </EmptyDescription>
            </EmptyHeader>
          </Empty>
        </div>
      )}
      {streaming && (
        <Badge className="absolute top-3 right-3 backdrop-blur-md" aria-label="Frames per second">
          {fps ?? "--"} FPS
        </Badge>
      )}
    </div>
  );
}
