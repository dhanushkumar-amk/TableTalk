import { useState } from "react"
import { Copy, Check, Clock, ShoppingBag, XCircle, AlertCircle, AlertTriangle } from "lucide-react"
import type { OrderSummary, PriceChange } from "../types"
import type { CardEventHandlers } from "../events"
import { AllergenNotice } from "./AllergenNotice"
import { SpokenCaption } from "./SpokenCaption"

interface OrderSummaryCardProps {
  order: OrderSummary
  priceChanges?: PriceChange[] | null
  allergenNotice?: string | null
  spokenText?: string | null
  events?: CardEventHandlers
}

export function OrderSummaryCard({
  order,
  priceChanges = [],
  allergenNotice,
  spokenText,
  events,
}: OrderSummaryCardProps) {
  const [copied, setCopied] = useState(false)
  const [showCancelConfirm, setShowCancelConfirm] = useState(false)

  const copyCode = async () => {
    try {
      await navigator.clipboard.writeText(order.order_code)
      setCopied(true)
      events?.onCopyCode?.({ code: order.order_code, label: "Order Code" })
      setTimeout(() => setCopied(false), 2000)
    } catch {
      setCopied(true)
      setTimeout(() => setCopied(false), 2000)
    }
  }

  const handleConfirmCancel = () => {
    events?.onCancelOrder?.({
      orderCode: order.order_code,
      guestName: order.guest_name,
    })
    setShowCancelConfirm(false)
  }

  const lines = order.lines || []
  const subtotalDisplay = order.subtotal?.display || "—"
  const taxDisplay = order.tax?.display || "—"
  const totalDisplay = order.total?.display || "—"

  return (
    <article
      aria-label={`Pickup order summary: ${order.order_code}`}
      className="flex flex-col space-y-4 rounded-2xl border border-border bg-card p-5 shadow-sm transition-all select-text"
    >
      {/* Header */}
      <header className="flex flex-wrap items-center justify-between gap-2 border-b border-border/60 pb-3">
        <div className="flex items-center gap-2">
          <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-primary/10 text-primary">
            <ShoppingBag className="h-4 w-4" aria-hidden="true" />
          </div>
          <div>
            <h2 className="text-sm font-semibold text-foreground">Pickup Order</h2>
            <p className="text-xs text-muted-foreground">{order.guest_name}</p>
          </div>
        </div>

        {/* Payment & Order Status */}
        <div className="flex items-center gap-1.5">
          <span className="rounded-full bg-secondary px-2.5 py-0.5 text-xs font-semibold text-secondary-foreground">
            Pay at pickup
          </span>
          <span className="rounded-full bg-emerald-500/15 px-2.5 py-0.5 text-xs font-semibold uppercase text-emerald-700 dark:text-emerald-300">
            {order.status}
          </span>
        </div>
      </header>

      {/* Order Code Box */}
      <div className="flex items-center justify-between rounded-xl border border-border/80 bg-muted/40 p-3.5">
        <div>
          <div className="text-[10px] font-mono uppercase tracking-wider text-muted-foreground">
            Order Code
          </div>
          <div className="font-mono text-xl font-bold tracking-wider text-foreground select-all">
            {order.order_code}
          </div>
        </div>

        <button
          type="button"
          onClick={copyCode}
          aria-label={copied ? "Code copied to clipboard" : "Copy order code"}
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

      {/* Pickup Time */}
      <div className="flex items-center gap-2 rounded-xl bg-accent/20 px-3.5 py-2.5 text-xs font-medium text-foreground">
        <Clock className="h-4 w-4 text-primary shrink-0" aria-hidden="true" />
        <span>
          Ready for pickup at{" "}
          <strong>
            {order.pickup_time_local
              ? new Date(order.pickup_time_local).toLocaleTimeString([], {
                  hour: "2-digit",
                  minute: "2-digit",
                })
              : "as scheduled"}
          </strong>
        </span>
      </div>

      {/* Price Changes Notice Banner */}
      {priceChanges && priceChanges.length > 0 && (
        <div className="rounded-xl border border-amber-500/30 bg-amber-500/10 p-3 text-xs text-amber-800 dark:text-amber-300 space-y-1">
          <div className="flex items-center gap-1.5 font-semibold">
            <AlertTriangle className="h-4 w-4" aria-hidden="true" />
            <span>Updated Menu Prices</span>
          </div>
          <p className="text-[11px] leading-relaxed">
            Some dish prices were updated on today's fresh menu since your last visit.
          </p>
        </div>
      )}

      {/* Line Items */}
      <div className="space-y-2">
        <h3 className="text-xs font-semibold text-foreground">Order Items</h3>
        <div className="divide-y divide-border/60 rounded-xl border border-border bg-card/70">
          {lines.map((line, idx) => (
            <div key={idx} className="flex items-center justify-between p-3 text-xs">
              <div className="space-y-0.5">
                <div className="font-medium text-foreground">
                  <span className="font-mono font-bold mr-1.5">{line.quantity}×</span>
                  <span>{line.name}</span>
                </div>
                {line.notes && (
                  <p className="text-[11px] text-muted-foreground italic">Note: {line.notes}</p>
                )}
                {line.allergens && line.allergens.length > 0 && (
                  <p className="text-[10px] text-muted-foreground">
                    Contains: {line.allergens.join(", ")}
                  </p>
                )}
              </div>
              <div className="font-mono font-semibold text-foreground">
                {line.line_total?.display || "—"}
              </div>
            </div>
          ))}
        </div>
      </div>

      {/* Pricing Breakdown */}
      <div className="rounded-xl border border-border/70 bg-muted/20 p-3 text-xs space-y-1.5">
        <div className="flex items-center justify-between text-muted-foreground">
          <span>Subtotal</span>
          <span className="font-mono">{subtotalDisplay}</span>
        </div>
        <div className="flex items-center justify-between text-muted-foreground">
          <span>Tax (5% GST)</span>
          <span className="font-mono">{taxDisplay}</span>
        </div>
        <div className="flex items-center justify-between border-t border-border/60 pt-1.5 text-sm font-bold text-foreground">
          <span>Total</span>
          <span className="font-mono">{totalDisplay}</span>
        </div>
      </div>

      {/* Stated Allergens or Kitchen Notice */}
      {allergenNotice && <AllergenNotice notice={allergenNotice} />}

      {/* Destructive Cancel Order Action with Confirm Step */}
      {order.status === "placed" && (
        <div className="border-t border-border/60 pt-3">
          {!showCancelConfirm ? (
            <button
              type="button"
              onClick={() => setShowCancelConfirm(true)}
              className="inline-flex min-h-[44px] cursor-pointer items-center gap-1.5 rounded-lg border border-border px-3 py-2 text-xs font-medium text-muted-foreground hover:bg-destructive/10 hover:text-destructive hover:border-destructive/30 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring transition-colors"
            >
              <XCircle className="h-4 w-4" aria-hidden="true" />
              <span>Cancel Order</span>
            </button>
          ) : (
            <div className="flex flex-col gap-2 rounded-xl border border-destructive/40 bg-destructive/5 p-3">
              <div className="flex items-center gap-1.5 text-xs font-semibold text-destructive">
                <AlertCircle className="h-4 w-4" aria-hidden="true" />
                <span>Confirm cancellation of this pickup order?</span>
              </div>
              <div className="flex items-center gap-2 pt-1">
                <button
                  type="button"
                  onClick={handleConfirmCancel}
                  className="min-h-[44px] cursor-pointer rounded-lg bg-destructive px-3 py-2 text-xs font-semibold text-destructive-foreground hover:bg-destructive/90 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring transition-colors"
                >
                  Yes, cancel order
                </button>
                <button
                  type="button"
                  onClick={() => setShowCancelConfirm(false)}
                  className="min-h-[44px] cursor-pointer rounded-lg border border-border bg-background px-3 py-2 text-xs font-medium text-foreground hover:bg-muted focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring transition-colors"
                >
                  Keep order
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
