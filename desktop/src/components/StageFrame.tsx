import { Empty, EmptyHeader, EmptyMedia, EmptyTitle, EmptyDescription } from "@/components/ui/empty";
import logoUrl from "@/assets/logo.png";

export function StageFrame({ streaming, fps }: { streaming: boolean; fps?: string }) {
  return (
    <div className="relative h-full max-h-full w-full overflow-hidden rounded-xl border border-white/8 bg-black shadow-2xl shadow-black/50">
      {/* Invariant D06: native preview renders to this surface; never passing frames through React.
          The native GStreamer waylandsink window overlays this exact rect. Inner area stays transparent
          and free of React children so surface lifecycle attachment never conflicts with re-renders. */}
      <div id="native-preview-surface-slot" className="absolute inset-0 bg-transparent" />
      {!streaming && (
        <div className="absolute inset-0">
          <Empty className="h-full">
            <EmptyHeader>
              <EmptyMedia variant="icon">
                <img src={logoUrl} alt="" className="size-full rounded-lg object-cover" />
              </EmptyMedia>
              <EmptyTitle>CamaPro Scope</EmptyTitle>
              <EmptyDescription>Connect your phone to start</EmptyDescription>
              <p className="pt-2 text-[#94a3b8] text-xs">
                Waiting for camera…
                <span className="block pt-0.5 opacity-70">Make sure your phone is connected</span>
              </p>
            </EmptyHeader>
          </Empty>
        </div>
      )}
      {streaming && (
        <span className="absolute top-3 right-3 flex items-center gap-1.5 rounded-md border border-white/10 bg-[#101826]/72 px-2 py-1 font-mono text-[11px] text-[#f5f7fa] backdrop-blur-[16px]">
          <span className="size-1.5 animate-pulse rounded-full bg-[#4ade80]" />
          {fps ?? "LIVE"}
        </span>
      )}
    </div>
  );
}
