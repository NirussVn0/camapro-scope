import { Card } from "@/components/ui/card";
import { cn } from "@/lib/utils";

export function Panel({ className, ...props }: React.ComponentProps<typeof Card>) {
  return (
    <Card
      className={cn(
        "glass-panel rounded-2xl border border-white/10 bg-white/5 shadow-none backdrop-blur-xl hover:bg-white/[0.08] dark",
        className,
      )}
      {...props}
    />
  );
}
