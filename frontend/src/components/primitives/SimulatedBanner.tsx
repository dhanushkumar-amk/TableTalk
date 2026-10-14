import { Info, ShieldAlert } from "lucide-react"
import { cn } from "@/lib/utils"

export interface SimulatedBannerProps {
  type?: "payment" | "preview" | "demo"
  message?: string
  className?: string
}

export function SimulatedBanner({
  type = "payment",
  message,
  className,
}: SimulatedBannerProps) {
  const isPayment = type === "payment"
  const defaultMessage = isPayment
    ? "SIMULATED PAYMENT — No real money moves. Demonstrating voice booking deposit capabilities."
    : "DEMO PREVIEW — Fictional restaurant environment for the Amazon Developer Hackathon."

  return (
    <div
      role="region"
      aria-label="Simulation Notice"
      className={cn(
        "flex items-center gap-3 rounded-xl border px-4 py-3 text-xs sm:text-sm font-medium",
        isPayment
          ? "border-amber-500/30 bg-amber-500/10 text-amber-900 dark:text-amber-200"
          : "border-primary/20 bg-primary/5 text-foreground",
        className
      )}
    >
      <div className="flex size-7 shrink-0 items-center justify-center rounded-lg bg-background/80 shadow-2xs border border-border/50">
        {isPayment ? (
          <ShieldAlert className="size-4 text-amber-600 dark:text-amber-400" aria-hidden="true" />
        ) : (
          <Info className="size-4 text-primary" aria-hidden="true" />
        )}
      </div>
      <p className="leading-snug">{message ?? defaultMessage}</p>
    </div>
  )
}
