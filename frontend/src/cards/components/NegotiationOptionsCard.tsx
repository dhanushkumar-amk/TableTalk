import { useState, useEffect } from "react"
import { Clock, ShieldCheck, Check, Timer } from "lucide-react"
import type { NegotiateBookingOutput, NegotiationOption } from "../types"
import type { CardEventHandlers } from "../events"
import { translateReasonCode } from "../reasonMapping"
import { SpokenCaption } from "./SpokenCaption"

interface NegotiationOptionsCardProps {
  structuredContent: NegotiateBookingOutput
  spokenText?: string | null
  events?: CardEventHandlers
}

/**
 * Formats seconds into MM:SS display string
 */
function formatCountdown(totalSeconds: number): string {
  if (totalSeconds <= 0) return "00:00"
  const mins = Math.floor(totalSeconds / 60)
  const secs = totalSeconds % 60
  return `${String(mins).padStart(2, "0")}:${String(secs).padStart(2, "0")}`
}

export function NegotiationOptionsCard({
  structuredContent,
  spokenText,
  events,
}: NegotiationOptionsCardProps) {
  const { options = [], status, message, round } = structuredContent

  // Find any active hold across options
  const heldOption = options.find((opt: any) => opt.hold?.seconds_remaining && opt.hold.seconds_remaining > 0)
  const initialSeconds = (heldOption as any)?.hold?.seconds_remaining ?? 0

  const [secondsRemaining, setSecondsRemaining] = useState<number>(initialSeconds)
  const [announcement, setAnnouncement] = useState<string>("")

  useEffect(() => {
    if (initialSeconds <= 0) return

    setSecondsRemaining(initialSeconds)
    const interval = setInterval(() => {
      setSecondsRemaining((prev) => {
        if (prev <= 1) {
          clearInterval(interval)
          setAnnouncement("The table hold has expired.")
          return 0
        }
        const next = prev - 1
        // Milestone announcements only (never every second) to satisfy WCAG live region rules
        if (next === 600) setAnnouncement("10 minutes remaining on table hold.")
        else if (next === 300) setAnnouncement("5 minutes remaining on table hold.")
        else if (next === 120) setAnnouncement("2 minutes remaining on table hold.")
        else if (next === 60) setAnnouncement("1 minute remaining on table hold.")
        return next
      })
    }, 1000)

    return () => clearInterval(interval)
  }, [initialSeconds])

  return (
    <section
      aria-label="Negotiated dining alternatives"
      className="flex flex-col space-y-4 rounded-2xl border border-border bg-card p-5 shadow-sm transition-all select-text"
    >
      {/* Screen reader live region for hold countdown milestones */}
      <div role="status" aria-live="polite" className="sr-only">
        {announcement}
      </div>

      {/* Header */}
      <header className="flex flex-wrap items-center justify-between gap-2 border-b border-border/60 pb-3">
        <div>
          <h2 className="text-sm font-semibold text-foreground">Dining Options</h2>
          <p className="text-xs text-muted-foreground">
            {message || `Ranked alternative tables (Round ${round ?? 1})`}
          </p>
        </div>

        {/* Hold Countdown Badge if an option is on hold */}
        {heldOption && (
          <div
            className={`flex items-center gap-1.5 rounded-full px-3 py-1 font-mono text-xs font-semibold ${
              secondsRemaining > 0
                ? "bg-amber-500/15 text-amber-700 dark:text-amber-300 border border-amber-500/30"
                : "bg-destructive/15 text-destructive border border-destructive/30"
            }`}
          >
            <Timer className="h-3.5 w-3.5 animate-pulse" aria-hidden="true" />
            <span>Hold: {formatCountdown(secondsRemaining)}</span>
          </div>
        )}
      </header>

      {/* Options List */}
      {options.length === 0 ? (
        <div className="py-6 text-center text-xs text-muted-foreground">
          {status === "no_viable_options"
            ? "No viable alternative tables could be found for this request."
            : "No alternative options available."}
        </div>
      ) : (
        <div className="space-y-3">
          {options.map((opt: NegotiationOption, idx: number) => {
            const hasHold = Boolean((opt as any).hold?.seconds_remaining && (opt as any).hold.seconds_remaining > 0)
            const reasons = opt.score_reasons || []

            return (
              <div
                key={opt.option_id || idx}
                className={`relative flex flex-col justify-between rounded-xl border p-4 transition-all ${
                  hasHold
                    ? "border-amber-500/50 bg-amber-500/5 shadow-xs"
                    : "border-border bg-card/80 hover:border-border/80"
                }`}
              >
                {/* Top Details */}
                <div className="flex items-start justify-between gap-2">
                  <div>
                    <div className="flex items-center gap-2">
                      <span className="font-semibold text-sm text-foreground">
                        {opt.start_local} – {opt.end_local}
                      </span>
                      <span className="rounded bg-secondary px-2 py-0.5 text-[11px] font-medium capitalize text-secondary-foreground">
                        {opt.area} Area
                      </span>
                      <span className="font-mono text-xs text-muted-foreground">
                        Table {opt.table_label}
                      </span>
                    </div>

                    {/* Human friendly reason chips */}
                    {reasons.length > 0 && (
                      <div className="mt-2 flex flex-wrap gap-1.5">
                        {reasons.map((r, rIdx) => (
                          <span
                            key={rIdx}
                            className="inline-flex items-center gap-1 rounded-md bg-muted px-2 py-0.5 text-[10px] text-muted-foreground"
                          >
                            <ShieldCheck className="h-2.5 w-2.5" aria-hidden="true" />
                            <span>{translateReasonCode(r)}</span>
                          </span>
                        ))}
                      </div>
                    )}
                  </div>

                  {/* Rank badge */}
                  <span className="shrink-0 rounded-full bg-accent px-2 py-0.5 font-mono text-[10px] font-semibold text-accent-foreground">
                    #{opt.rank ?? idx + 1}
                  </span>
                </div>

                {/* Action Buttons */}
                <div className="mt-3.5 flex flex-wrap items-center justify-end gap-2 border-t border-border/40 pt-2.5">
                  {/* Hold Button */}
                  {!hasHold && (
                    <button
                      type="button"
                      onClick={() => events?.onHoldOption?.({ optionId: opt.option_id })}
                      className="inline-flex min-h-[44px] cursor-pointer items-center gap-1.5 rounded-lg border border-border bg-background px-3 py-2 text-xs font-medium text-foreground hover:bg-muted focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring transition-colors"
                    >
                      <Clock className="h-3.5 w-3.5" aria-hidden="true" />
                      <span>Hold 15 min</span>
                    </button>
                  )}

                  {/* Accept Button */}
                  <button
                    type="button"
                    onClick={() =>
                      events?.onAcceptOption?.({
                        optionId: opt.option_id,
                      })
                    }
                    className="inline-flex min-h-[44px] cursor-pointer items-center gap-1.5 rounded-lg bg-primary px-4 py-2 text-xs font-semibold text-primary-foreground hover:bg-primary/90 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring transition-colors shadow-xs"
                  >
                    <Check className="h-4 w-4" aria-hidden="true" />
                    <span>Accept Option</span>
                  </button>
                </div>
              </div>
            )
          })}
        </div>
      )}

      {/* Secondary transcript caption */}
      <SpokenCaption text={spokenText} />
    </section>
  )
}
