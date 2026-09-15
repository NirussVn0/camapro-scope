import { Card } from "@/components/ui/card";
import { cn } from "@/lib/utils";

export function Panel({ className, ...props }: React.ComponentProps<typeof Card>) {
  return (
    <Card
      className={cn(
        "glass-panel rounded-xl border border-white/8 bg-[#101826]/72 shadow-sm shadow-black/30 backdrop-blur-[16px]",
        className,
      )}
      {...props}
    />
  );
}
