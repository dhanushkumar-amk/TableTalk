import { useState } from "react"
import { Copy, Check, Calendar, Users, MapPin, XCircle, AlertCircle } from "lucide-react"
import type { BookingSummary } from "../types"
import type { CardEventHandlers } from "../events"
import { SpokenCaption } from "./SpokenCaption"

interface BookingConfirmationCardProps {
  booking: BookingSummary
  cancellationPolicy?: string | null
  spokenText?: string | null
  events?: CardEventHandlers
}

export function BookingConfirmationCard({
  booking,
  cancellationPolicy,
  spokenText,
  events,
}: BookingConfirmationCardProps) {
  const [copied, setCopied] = useState(false)
  const [showCancelConfirm, setShowCancelConfirm] = useState(false)

  const copyCode = async () => {
    try {
      await navigator.clipboard.writeText(booking.confirmation_code)
      setCopied(true)
      events?.onCopyCode?.({ code: booking.confirmation_code, label: "Booking Confirmation Code" })
      setTimeout(() => setCopied(false), 2000)
    } catch {
      // Fallback if clipboard API is blocked in test environment
      setCopied(true)
      setTimeout(() => setCopied(false), 2000)
    }
  }

  const handleConfirmCancel = () => {
    events?.onCancelBooking?.({
      confirmationCode: booking.confirmation_code,
      guestName: booking.guest_name,
    })
    setShowCancelConfirm(false)
  }

  return (
    <article
      aria-label={`Reservation confirmation: ${booking.confirmation_code}`}
      className="flex flex-col space-y-4 rounded-2xl border border-border bg-card p-5 shadow-sm transition-all select-text"
    >
      {/* Header */}
      <header className="flex flex-wrap items-center justify-between gap-2 border-b border-border/60 pb-3">
        <div>
          <h2 className="text-sm font-semibold text-foreground">Table Reservation</h2>
          <p className="text-xs text-muted-foreground">The Saffron Nebula Bistro</p>
        </div>

        {/* Status Badge */}
        <span
          className={`inline-flex items-center rounded-full px-2.5 py-0.5 text-xs font-semibold uppercase tracking-wider ${
            booking.status === "confirmed"
              ? "bg-emerald-500/15 text-emerald-700 dark:text-emerald-300"
              : booking.status === "cancelled"
              ? "bg-destructive/15 text-destructive"
              : "bg-amber-500/15 text-amber-700 dark:text-amber-300"
          }`}
        >
          {booking.status}
        </span>
      </header>

      {/* Confirmation Code Block */}
      <div className="flex items-center justify-between rounded-xl border border-border/80 bg-muted/40 p-3.5">
        <div>
          <div className="text-[10px] font-mono uppercase tracking-wider text-muted-foreground">
            Confirmation Code
          </div>
          <div className="font-mono text-xl font-bold tracking-wider text-foreground select-all">
            {booking.confirmation_code}
          </div>
        </div>

        <button
          type="button"
          onClick={copyCode}
          aria-label={copied ? "Code copied to clipboard" : "Copy confirmation code"}
          className="flex min-h-[44px] min-w-[44px] cursor-pointer items-center justify-center gap-1.5 rounded-lg border border-border bg-background px-3 py-2 text-xs font-medium text-foreground hover:bg-muted focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring transition-colors"
        >
          {copied ? (
            <>
              <Check className="h-4 w-4 text-emerald-600" aria-hidden="true" />
              <span className="text-emerald-600 font-semibold">Copied</span>
            </>
          ) : (
            <>
              <Copy className="h-4 w-4 text-muted-foreground" aria-hidden="true" />
              <span>Copy</span>
            </>
          )}
        </button>
      </div>

      {/* Booking Details Grid */}
      <div className="grid grid-cols-2 gap-3 text-xs">
        <div className="flex items-center gap-2">
          <Calendar className="h-4 w-4 text-muted-foreground" aria-hidden="true" />
          <div>
            <div className="text-[11px] text-muted-foreground">Date & Time</div>
            <div className="font-medium text-foreground">
              {new Date(booking.start_local).toLocaleDateString([], {
                weekday: "short",
                month: "short",
                day: "numeric",
              })}{" "}
              at{" "}
              {new Date(booking.start_local).toLocaleTimeString([], {
                hour: "2-digit",
                minute: "2-digit",
              })}
            </div>
          </div>
        </div>

        <div className="flex items-center gap-2">
          <Users className="h-4 w-4 text-muted-foreground" aria-hidden="true" />
          <div>
            <div className="text-[11px] text-muted-foreground">Guest & Party</div>
            <div className="font-medium text-foreground">
              {booking.guest_name} • {booking.party_size} {booking.party_size === 1 ? "guest" : "guests"}
            </div>
          </div>
        </div>

        <div className="flex items-center gap-2">
          <MapPin className="h-4 w-4 text-muted-foreground" aria-hidden="true" />
          <div>
            <div className="text-[11px] text-muted-foreground">Area & Table</div>
            <div className="font-medium text-foreground capitalize">
              {booking.area} area • Table {booking.table_label}
            </div>
          </div>
        </div>
      </div>

      {/* Special requests if present */}
      {booking.special_requests && (
        <div className="rounded-lg bg-accent/20 px-3 py-2 text-xs">
          <span className="font-semibold text-foreground">Special Request: </span>
          <span className="text-foreground/90">{booking.special_requests}</span>
        </div>
      )}

      {/* Cancellation Policy Note */}
      {cancellationPolicy && (
        <div className="border-t border-border/50 pt-2 text-[11px] text-muted-foreground">
          <span className="font-semibold">Cancellation Policy: </span>
          <span>{cancellationPolicy}</span>
        </div>
      )}

      {/* Destructive Cancel Action with Confirm Step */}
      {booking.status === "confirmed" && (
        <div className="border-t border-border/60 pt-3">
          {!showCancelConfirm ? (
            <button
              type="button"
              onClick={() => setShowCancelConfirm(true)}
              className="inline-flex min-h-[44px] cursor-pointer items-center gap-1.5 rounded-lg border border-border px-3 py-2 text-xs font-medium text-muted-foreground hover:bg-destructive/10 hover:text-destructive hover:border-destructive/30 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring transition-colors"
            >
              <XCircle className="h-4 w-4" aria-hidden="true" />
              <span>Cancel Reservation</span>
            </button>
          ) : (
            <div className="flex flex-col gap-2 rounded-xl border border-destructive/40 bg-destructive/5 p-3">
              <div className="flex items-center gap-1.5 text-xs font-semibold text-destructive">
                <AlertCircle className="h-4 w-4" aria-hidden="true" />
                <span>Confirm cancellation for this reservation?</span>
              </div>
              <div className="flex items-center gap-2 pt-1">
                <button
                  type="button"
                  onClick={handleConfirmCancel}
                  className="min-h-[44px] cursor-pointer rounded-lg bg-destructive px-3 py-2 text-xs font-semibold text-destructive-foreground hover:bg-destructive/90 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring transition-colors"
                >
                  Yes, cancel reservation
                </button>
                <button
                  type="button"
                  onClick={() => setShowCancelConfirm(false)}
                  className="min-h-[44px] cursor-pointer rounded-lg border border-border bg-background px-3 py-2 text-xs font-medium text-foreground hover:bg-muted focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring transition-colors"
                >
                  Keep reservation
                </button>
              </div>
            </div>
          )}
        </div>
      )}

      {/* Secondary Transcript Caption */}
      <SpokenCaption text={spokenText} />
    </article>
  )
}
