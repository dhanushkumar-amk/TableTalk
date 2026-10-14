import { useState, useEffect } from "react"
import { ShieldAlert, Timer, ArrowUpRight, Lock, CheckCircle2, RotateCcw } from "lucide-react"
import type { PaymentObject, GetDepositStatusOutput } from "../types"
import type { CardEventHandlers } from "../events"
import { SpokenCaption } from "./SpokenCaption"

interface PaymentCardProps {
  payment?: PaymentObject | null
  depositStatusOutput?: GetDepositStatusOutput | null
  spokenText?: string | null
  events?: CardEventHandlers
}

function formatCountdown(totalSeconds: number): string {
  if (totalSeconds <= 0) return "00:00"
  const mins = Math.floor(totalSeconds / 60)
  const secs = totalSeconds % 60
  return `${String(mins).padStart(2, "0")}:${String(secs).padStart(2, "0")}`
}

export function PaymentCard({
  payment,
  depositStatusOutput,
  spokenText,
  events,
}: PaymentCardProps) {
  // Amount display
  const amountDisplay =
    payment?.amount?.display ??
    depositStatusOutput?.amount?.display ??
    "—"

  // Status
  const status =
    depositStatusOutput?.deposit_status ??
    depositStatusOutput?.status ??
    payment?.status ??
    "pending"

  const isPaid = status === "paid" || status === "success" || status === "captured"

  // Deadline countdown (minutes to seconds)
  const initialMinutes =
    payment?.minutes_remaining ??
    depositStatusOutput?.minutes_remaining ??
    (payment?.deadline_local ? 15 : 0)
  const initialSeconds = initialMinutes * 60

  const [secondsRemaining, setSecondsRemaining] = useState<number>(initialSeconds)

  useEffect(() => {
    if (initialSeconds <= 0 || isPaid) return

    setSecondsRemaining(initialSeconds)
    const interval = setInterval(() => {
      setSecondsRemaining((prev) => {
        if (prev <= 1) {
          clearInterval(interval)
          return 0
        }
        return prev - 1
      })
    }, 1000)

    return () => clearInterval(interval)
  }, [initialSeconds, isPaid])

  const checkoutUrl = payment?.checkout_url

  const handleOpenCheckout = () => {
    if (checkoutUrl) {
      events?.onOpenCheckout?.({ checkoutUrl })
    }
  }

  return (
    <article
      aria-label="Simulated table deposit payment"
      className="flex flex-col space-y-4 rounded-2xl border border-border bg-card p-5 shadow-sm transition-all select-text"
    >
      {/* Prominent Simulated Payment Warning Banner */}
      <div
        role="alert"
        className="flex items-center gap-2 rounded-xl border border-amber-500/40 bg-amber-500/10 px-3.5 py-2 text-xs font-semibold text-amber-700 dark:text-amber-400"
      >
        <ShieldAlert className="h-4 w-4 shrink-0 text-amber-600 dark:text-amber-400" aria-hidden="true" />
        <span>SIMULATED PAYMENT — no real money</span>
      </div>

      {/* Header */}
      <header className="flex flex-wrap items-center justify-between gap-2 border-b border-border/60 pb-3">
        <div>
          <h2 className="text-sm font-semibold text-foreground">Table Deposit</h2>
          <p className="text-xs text-muted-foreground">The Saffron Nebula Bistro</p>
        </div>

        {/* Status Badge */}
        <span
          className={`inline-flex items-center gap-1 rounded-full px-2.5 py-0.5 text-xs font-semibold uppercase tracking-wider ${
            isPaid
              ? "bg-emerald-500/15 text-emerald-700 dark:text-emerald-300"
              : status === "expired" || status === "declined"
              ? "bg-destructive/15 text-destructive"
              : "bg-amber-500/15 text-amber-700 dark:text-amber-300"
          }`}
        >
          {isPaid && <CheckCircle2 className="h-3 w-3" aria-hidden="true" />}
          <span>{status}</span>
        </span>
      </header>

      {/* Amount Display */}
      <div className="rounded-xl border border-border/70 bg-muted/20 p-4 text-center space-y-1">
        <div className="text-xs font-medium text-muted-foreground">Deposit Required</div>
        <div className="font-mono text-3xl font-bold tracking-tight text-foreground">
          {amountDisplay}
        </div>
        {!isPaid && secondsRemaining > 0 && (
          <div className="flex items-center justify-center gap-1.5 pt-1 text-xs text-amber-700 dark:text-amber-400 font-medium">
            <Timer className="h-3.5 w-3.5 animate-pulse" aria-hidden="true" />
            <span>Pay within {formatCountdown(secondsRemaining)} to guarantee reservation</span>
          </div>
        )}
      </div>

      {/* Refund Schedule / Policy */}
      {depositStatusOutput?.refund_if_cancelled_now && (
        <div className="rounded-xl border border-border/60 bg-muted/30 p-3 text-xs space-y-1">
          <div className="flex items-center gap-1 font-semibold text-foreground">
            <RotateCcw className="h-3.5 w-3.5" aria-hidden="true" />
            <span>Cancellation Refund Policy</span>
          </div>
          <p className="text-muted-foreground leading-relaxed">
            {depositStatusOutput.refund_if_cancelled_now.explanation} (
            {depositStatusOutput.refund_if_cancelled_now.percent_display} refund eligible now)
          </p>
        </div>
      )}

      {/* Mock Checkout Action Button */}
      {!isPaid && checkoutUrl && (
        <div className="pt-1">
          <button
            type="button"
            onClick={handleOpenCheckout}
            aria-label="Pay deposit (simulated) on mock checkout page"
            className="flex min-h-[44px] w-full cursor-pointer items-center justify-center gap-2 rounded-xl bg-primary px-4 py-2.5 text-xs font-semibold text-primary-foreground hover:bg-primary/90 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring transition-colors shadow-xs"
          >
            <span>Pay deposit (simulated)</span>
            <ArrowUpRight className="h-4 w-4" aria-hidden="true" />
          </button>
          <div className="mt-2 flex items-center justify-center gap-1 text-[11px] text-muted-foreground">
            <Lock className="h-3 w-3" aria-hidden="true" />
            <span>No card form or payment fields requested. Hosted mock ledger.</span>
          </div>
        </div>
      )}

      {/* Spoken Transcript Caption */}
      <SpokenCaption text={spokenText} />
    </article>
  )
}
