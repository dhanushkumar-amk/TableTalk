import { Trophy, Zap } from "lucide-react"

/**
 * Top Hackathon & Live Event Announcement Ribbon inspired by Screenshot 1.
 * Features angled badge stickers, laurel wreath achievement badge,
 * and live system metrics.
 */
export function EventHeroBanner() {
  return (
    <div className="relative overflow-hidden rounded-2xl border border-primary/30 bg-gradient-to-r from-card via-primary/5 to-muted/40 p-4 sm:p-5 shadow-xs mb-8 transition-all hover:border-primary/50">
      {/* Background subtle diagonal texture */}
      <div
        className="pointer-events-none absolute inset-0 opacity-[0.03] dark:opacity-[0.06]"
        style={{
          backgroundImage: "repeating-linear-gradient(45deg, currentColor 0, currentColor 1px, transparent 0, transparent 16px)",
        }}
      />

      <div className="relative flex flex-col lg:flex-row items-start lg:items-center justify-between gap-4">
        {/* Left: Event Tags & Title Badge */}
        <div className="flex flex-wrap items-center gap-3">
          <div className="inline-flex items-center gap-1.5 px-3 py-1 rounded-md bg-primary text-primary-foreground text-xs font-semibold tracking-wide uppercase shadow-2xs">
            <span className="font-mono">EVENT 02</span>
            <span>·</span>
            <span>ALEXA+ & MCP TOUR</span>
          </div>

          <div className="inline-flex items-center gap-1.5 px-2.5 py-0.5 rounded-full border border-border bg-background/80 text-xs text-foreground font-mono">
            <span className="size-2 rounded-full bg-emerald-500 animate-ping" />
            <span>OCT 8 – 11, 2026</span>
          </div>

          <span className="text-xs text-muted-foreground hidden sm:inline">
            Fast-track voice-first restaurant intelligence
          </span>
        </div>

        {/* Right: Badges & Prize Wreath Sticker */}
        <div className="flex flex-wrap items-center gap-3">
          {/* Laurel Wreath Prize / Tools Sticker */}
          <div className="inline-flex items-center gap-2 px-3 py-1.5 rounded-xl border border-emerald-500/30 bg-emerald-500/10 text-emerald-800 dark:text-emerald-300 text-xs font-medium">
            <Trophy className="size-4 text-emerald-600 dark:text-emerald-400 shrink-0" />
            <div>
              <span className="font-bold font-mono tracking-tight text-foreground block text-[13px] leading-tight">
                24 Rust MCP Tools
              </span>
              <span className="text-[10px] text-muted-foreground block">
                Sub-30ms Latency · Zero Remote Fonts
              </span>
            </div>
          </div>

          {/* Fast-Track Badge */}
          <div className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-xl border border-primary/30 bg-background text-xs font-semibold text-foreground shadow-2xs">
            <Zap className="size-3.5 text-primary shrink-0" />
            <span>Fast-Track AI Execution</span>
          </div>
        </div>
      </div>
    </div>
  )
}
