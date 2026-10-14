import { Calendar, Clock, Users, MapPin, CheckCircle, XCircle, AlertTriangle } from "lucide-react"
import type { CheckAvailabilityOutput, AlternativeSlot } from "../types"
import type { CardEventHandlers } from "../events"
import { translateReasonCode } from "../reasonMapping"
import { SpokenCaption } from "./SpokenCaption"

interface AvailabilityCardProps {
  structuredContent: CheckAvailabilityOutput
  spokenText?: string | null
  events?: CardEventHandlers
}

export function AvailabilityCard({ structuredContent, spokenText, events }: AvailabilityCardProps) {
  const { status, slot, alternatives, opening_hours_that_day, requested, reason, reason_detail, note } =
    structuredContent

  const isAvailable = status === "available" && slot !== null && slot !== undefined

  return (
    <article
      aria-label={`Availability check: ${status}`}
      className="flex flex-col space-y-4 rounded-2xl border border-border bg-card p-5 shadow-sm transition-all select-text"
    >
      {/* Header and Status */}
      <header className="flex flex-wrap items-center justify-between gap-2 border-b border-border/60 pb-3">
        <div className="flex items-center gap-2">
          {isAvailable ? (
            <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-emerald-500/10 text-emerald-600 dark:text-emerald-400">
              <CheckCircle className="h-4 w-4" aria-hidden="true" />
            </div>
          ) : status === "closed" ? (
            <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-amber-500/10 text-amber-600 dark:text-amber-400">
              <AlertTriangle className="h-4 w-4" aria-hidden="true" />
            </div>
          ) : (
            <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-rose-500/10 text-rose-600 dark:text-rose-400">
              <XCircle className="h-4 w-4" aria-hidden="true" />
            </div>
          )}
          <div>
            <h2 className="text-sm font-semibold text-foreground">Table Availability</h2>
            <p className="text-xs text-muted-foreground">
              {requested.date} at {requested.time} • Party of {requested.party_size}
            </p>
          </div>
        </div>

        {/* Status Badge */}
        <span
          className={`inline-flex items-center rounded-full px-2.5 py-0.5 text-xs font-semibold uppercase tracking-wider ${
            isAvailable
              ? "bg-emerald-500/15 text-emerald-700 dark:text-emerald-300"
              : status === "closed"
              ? "bg-amber-500/15 text-amber-700 dark:text-amber-300"
              : "bg-rose-500/15 text-rose-700 dark:text-rose-300"
          }`}
        >
          {status.replace("_", " ")}
        </span>
      </header>

      {/* Main Outcome Content */}
      {isAvailable && slot ? (
        <div className="rounded-xl border border-border/70 bg-accent/20 p-4 space-y-2">
          <div className="flex items-center justify-between">
            <span className="text-xs font-semibold text-foreground">Allocated Table</span>
            <span className="font-mono text-sm font-bold text-primary">{slot.table_label}</span>
          </div>
          <div className="grid grid-cols-2 gap-2 pt-1 text-xs text-muted-foreground">
            <div className="flex items-center gap-1.5">
              <Clock className="h-3.5 w-3.5 text-foreground/60" aria-hidden="true" />
              <span>
                {new Date(slot.start_local).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })} –{" "}
                {new Date(slot.end_local).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })}
              </span>
            </div>
            <div className="flex items-center gap-1.5">
              <MapPin className="h-3.5 w-3.5 text-foreground/60" aria-hidden="true" />
              <span className="capitalize">{slot.area} Area</span>
            </div>
            <div className="flex items-center gap-1.5">
              <Users className="h-3.5 w-3.5 text-foreground/60" aria-hidden="true" />
              <span>Fits up to {slot.capacity} guests</span>
            </div>
          </div>
        </div>
      ) : (
        <div className="rounded-xl border border-border/70 bg-muted/30 p-3.5 space-y-1.5">
          <div className="font-medium text-xs text-foreground">
            {translateReasonCode(reason) || "Selected time is currently unavailable"}
          </div>
          {reason_detail && (
            <p className="text-xs text-muted-foreground leading-relaxed">{reason_detail}</p>
          )}
        </div>
      )}

      {/* Alternative Slots */}
      {alternatives && alternatives.length > 0 && (
        <div className="space-y-2">
          <h3 className="text-xs font-semibold text-foreground flex items-center gap-1">
            <Clock className="h-3.5 w-3.5" aria-hidden="true" />
            <span>Suggested Alternative Slots</span>
          </h3>
          <div className="flex flex-wrap gap-2">
            {alternatives.map((alt: AlternativeSlot, idx: number) => {
              const timeLabel = new Date(alt.start_local).toLocaleTimeString([], {
                hour: "2-digit",
                minute: "2-digit",
              })
              const dateLabel = new Date(alt.start_local).toISOString().slice(0, 10)
              return (
                <button
                  key={idx}
                  type="button"
                  onClick={() =>
                    events?.onSelectAlternative?.({
                      date: dateLabel,
                      time: timeLabel,
                      area: alt.area,
                    })
                  }
                  className="inline-flex cursor-pointer items-center gap-1.5 rounded-lg border border-border bg-background px-3 py-1.5 text-xs font-medium text-foreground hover:bg-muted hover:border-border/80 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring transition-colors shadow-2xs"
                >
                  <span className="font-semibold">{timeLabel}</span>
                  <span className="text-muted-foreground capitalize">({alt.area})</span>
                  {alt.different_day && (
                    <span className="rounded bg-accent px-1 text-[10px] text-accent-foreground font-mono">
                      Next Day
                    </span>
                  )}
                </button>
              )
            })}
          </div>
        </div>
      )}

      {/* Opening Hours That Day */}
      {opening_hours_that_day && opening_hours_that_day.length > 0 && (
        <div className="border-t border-border/50 pt-2 text-[11px] text-muted-foreground flex items-center justify-between flex-wrap gap-1">
          <span className="flex items-center gap-1">
            <Calendar className="h-3.5 w-3.5" aria-hidden="true" />
            <span>Service Hours:</span>
          </span>
          <span className="font-medium text-foreground/80">
            {opening_hours_that_day.map((h) => `${h.open} – ${h.close}`).join("  |  ")}
          </span>
        </div>
      )}

      {/* Snapshot Disclaimer Note */}
      {note && (
        <p className="text-[11px] italic text-muted-foreground">
          ℹ️ {note}
        </p>
      )}

      {/* Transcript Caption */}
      <SpokenCaption text={spokenText} />
    </article>
  )
}
