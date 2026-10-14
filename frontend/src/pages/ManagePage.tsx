import { useState, useEffect } from "react"
import { useSearchParams } from "react-router-dom"
import { motion, AnimatePresence, useReducedMotion } from "framer-motion"
import {
  CalendarCheck,
  ShoppingBag,
  Users,
  Search,
  AlertCircle,
  ShieldAlert,
} from "lucide-react"
import { toast } from "sonner"
import { Button } from "@/components/ui/button"
import { Money } from "@/components/primitives/Money"
import { CodeBlock } from "@/components/primitives/CodeBlock"
import { StatusBadge } from "@/components/primitives/StatusBadge"
import { CountdownChip } from "@/components/primitives/CountdownChip"

type ManageTab = "booking" | "order" | "waitlist"

export function ManagePage() {
  const [searchParams, setSearchParams] = useSearchParams()
  const shouldReduceMotion = useReducedMotion()

  // Tab selection
  const activeTab: ManageTab = (searchParams.get("tab") as ManageTab) || "booking"
  const setTab = (tab: ManageTab) => {
    setSearchParams((prev) => {
      const next = new URLSearchParams(prev)
      next.set("tab", tab)
      return next
    })
    setLookupResult(null)
    setLookupError(null)
  }

  // Form state
  const [lookupCode, setLookupCode] = useState(
    searchParams.get("code") || searchParams.get("order") || ""
  )
  const [guestName, setGuestName] = useState(searchParams.get("name") || "")
  const [isLoading, setIsLoading] = useState(false)
  const [lookupError, setLookupError] = useState<string | null>(null)
  const [lookupAttempts, setLookupAttempts] = useState(0)

  // Result state
  const [lookupResult, setLookupResult] = useState<{
    type: "booking" | "order" | "waitlist"
    data: any
  } | null>(null)

  // Confirmation dialog state
  const [confirmDialog, setConfirmDialog] = useState<{
    open: boolean
    title: string
    description: string
    confirmLabel: string
    onConfirm: () => void
  } | null>(null)

  // Auto lookup if code is in URL on mount
  useEffect(() => {
    const code = searchParams.get("code") || searchParams.get("order")
    if (code && !lookupResult && !lookupError) {
      handleLookup(code, guestName || "Guest")
    }
  }, [])

  // Execute lookup
  const handleLookup = async (codeToLookup?: string, nameToLookup?: string) => {
    const code = (codeToLookup ?? lookupCode).trim()
    const name = (nameToLookup ?? guestName).trim()

    if (!code || !name) {
      setLookupError("Please enter both the confirmation code and the guest name.")
      return
    }

    // Rate-limiting security check (max 5 rapid attempts)
    if (lookupAttempts >= 5) {
      setLookupError(
        "Too many lookup attempts. For security reasons, please wait a moment before trying again."
      )
      return
    }

    setLookupAttempts((prev) => prev + 1)
    setIsLoading(true)
    setLookupError(null)

    // Simulate network lookup
    await new Promise((r) => setTimeout(r, 200))
    setIsLoading(false)

    // Not-found test case
    if (code.toLowerCase() === "invalid" || name.toLowerCase() === "unknown") {
      // Security Rule: NEVER reveal whether the code or the name was wrong!
      setLookupError(
        "No matching record found. Please verify the confirmation code and guest name on your confirmation."
      )
      setLookupResult(null)
      return
    }

    // Successful results depending on tab
    if (activeTab === "booking") {
      setLookupResult({
        type: "booking",
        data: {
          code: code || "TT-849201",
          guestName: name || "Priya Sharma",
          status: "confirmed",
          date: "Friday, Oct 16, 2026",
          time: "7:30 PM",
          partySize: 4,
          area: "Indoor Dining",
          tableLabel: "T01",
          depositAmountMinor: 0,
          cancellationPolicy:
            "Free cancellation up to 6 hours prior to your dining reservation.",
        },
      })
    } else if (activeTab === "order") {
      setLookupResult({
        type: "order",
        data: {
          code: code || "TT-ORD-1092",
          guestName: name || "Vikram Malhotra",
          status: "preparing", // 'placed' | 'preparing' | 'ready' | 'completed'
          pickupTime: "7:55 PM (in ~20 mins)",
          lines: [
            { name: "Dum Gosht Biryani", quantity: 1, priceMinor: 68000 },
            { name: "Tandoori Roti", quantity: 2, priceMinor: 18000 },
            { name: "Nebula Dal Makhani", quantity: 1, priceMinor: 42000 },
          ],
          totalMinor: 134400,
        },
      })
    } else {
      setLookupResult({
        type: "waitlist",
        data: {
          code: code || "WL-204",
          guestName: name || "Ananya Roy",
          partySize: 2,
          area: "Indoor",
          position: 2,
          estimatedWaitMinutes: 20,
          offerActive: true,
          offerExpiresAt: new Date(Date.now() + 5 * 60 * 1000).toISOString(),
        },
      })
    }
  }

  // Cancel Booking
  const handleCancelBooking = () => {
    setConfirmDialog({
      open: true,
      title: "Cancel Reservation?",
      description:
        "Are you sure you want to cancel this booking? Since this reservation required no deposit, no cancellation fees will apply.",
      confirmLabel: "Confirm Cancellation",
      onConfirm: () => {
        setConfirmDialog(null)
        setLookupResult((prev) =>
          prev ? { ...prev, data: { ...prev.data, status: "cancelled" } } : null
        )
        toast.success("Reservation cancelled successfully")
      },
    })
  }

  // Cancel Order
  const handleCancelOrder = () => {
    setConfirmDialog({
      open: true,
      title: "Cancel Pickup Order?",
      description:
        "Are you sure you want to cancel this pickup order? The kitchen will stop preparing your meal.",
      confirmLabel: "Cancel Order",
      onConfirm: () => {
        setConfirmDialog(null)
        setLookupResult((prev) =>
          prev ? { ...prev, data: { ...prev.data, status: "cancelled" } } : null
        )
        toast.info("Pickup order cancelled")
      },
    })
  }

  // Accept Waitlist Offer
  const handleAcceptOffer = () => {
    toast.success("Table offer accepted! Booking confirmed.")
    setLookupResult((prev) =>
      prev
        ? {
            type: "booking",
            data: {
              code: "TT-WL-CONF",
              guestName: prev.data.guestName,
              status: "confirmed",
              date: "Tonight",
              time: "Immediate Seating",
              partySize: prev.data.partySize,
              area: prev.data.area,
              tableLabel: "T02",
              depositAmountMinor: 0,
              cancellationPolicy: "Direct immediate seating confirmed.",
            },
          }
        : null
    )
  }

  // Leave Waitlist
  const handleLeaveWaitlist = () => {
    setConfirmDialog({
      open: true,
      title: "Leave Waitlist?",
      description: "Are you sure you want to forfeit your waitlist queue position?",
      confirmLabel: "Leave Waitlist",
      onConfirm: () => {
        setConfirmDialog(null)
        setLookupResult(null)
        toast.info("You have been removed from the waitlist.")
      },
    })
  }

  return (
    <div className="relative min-h-[calc(100vh-3.5rem)] bg-background pb-20">
      {/* Top Header */}
      <div className="border-b border-border/80 bg-card/40 backdrop-blur-xs px-4 py-6">
        <div className="mx-auto max-w-2xl text-center space-y-2">
          <h1 className="text-2xl sm:text-3xl font-semibold tracking-tight text-foreground">
            Manage Reservations & Orders
          </h1>
          <p className="text-xs sm:text-sm text-muted-foreground max-w-md mx-auto leading-relaxed">
            Look up table bookings, track pickup order progress, or monitor your waitlist position.
          </p>
        </div>
      </div>

      <div className="mx-auto max-w-2xl px-4 pt-8 space-y-6">
        {/* ========================================================= */}
        {/* 1. TABS CARD                                              */}
        {/* ========================================================= */}
        <div className="rounded-2xl border border-border/80 bg-card shadow-xs overflow-hidden">
          {/* 3 Navigation Tabs */}
          <div className="grid grid-cols-3 border-b border-border/80 bg-muted/20" role="tablist">
            <button
              type="button"
              role="tab"
              aria-selected={activeTab === "booking"}
              onClick={() => setTab("booking")}
              className={`flex items-center justify-center gap-2 py-3.5 text-xs sm:text-sm font-medium transition-all ${
                activeTab === "booking"
                  ? "border-b-2 border-primary bg-card text-foreground font-semibold"
                  : "text-muted-foreground hover:text-foreground"
              }`}
            >
              <CalendarCheck className="size-4 text-primary" />
              <span>Booking</span>
            </button>

            <button
              type="button"
              role="tab"
              aria-selected={activeTab === "order"}
              onClick={() => setTab("order")}
              className={`flex items-center justify-center gap-2 py-3.5 text-xs sm:text-sm font-medium transition-all ${
                activeTab === "order"
                  ? "border-b-2 border-primary bg-card text-foreground font-semibold"
                  : "text-muted-foreground hover:text-foreground"
              }`}
            >
              <ShoppingBag className="size-4 text-primary" />
              <span>Order</span>
            </button>

            <button
              type="button"
              role="tab"
              aria-selected={activeTab === "waitlist"}
              onClick={() => setTab("waitlist")}
              className={`flex items-center justify-center gap-2 py-3.5 text-xs sm:text-sm font-medium transition-all ${
                activeTab === "waitlist"
                  ? "border-b-2 border-primary bg-card text-foreground font-semibold"
                  : "text-muted-foreground hover:text-foreground"
              }`}
            >
              <Users className="size-4 text-primary" />
              <span>Waitlist</span>
            </button>
          </div>

          {/* Lookup Form */}
          <div className="p-6 space-y-4">
            <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
              <div className="space-y-1.5">
                <label htmlFor="lookupCode" className="text-xs font-medium text-foreground">
                  {activeTab === "booking"
                    ? "Confirmation Code"
                    : activeTab === "order"
                      ? "Pickup Order Code"
                      : "Waitlist Code"}
                </label>
                <input
                  id="lookupCode"
                  type="text"
                  value={lookupCode}
                  onChange={(e) => setLookupCode(e.target.value)}
                  placeholder={
                    activeTab === "booking"
                      ? "e.g. TT-849201"
                      : activeTab === "order"
                        ? "e.g. TT-ORD-1092"
                        : "e.g. WL-204"
                  }
                  className="h-10 w-full rounded-xl border border-border/80 bg-muted/20 px-3.5 text-sm text-foreground placeholder:text-muted-foreground focus:border-ring focus:outline-none focus:ring-2 focus:ring-ring/20"
                />
              </div>

              <div className="space-y-1.5">
                <label htmlFor="guestName" className="text-xs font-medium text-foreground">
                  Guest Name
                </label>
                <input
                  id="guestName"
                  type="text"
                  value={guestName}
                  onChange={(e) => setGuestName(e.target.value)}
                  placeholder="e.g. Priya Sharma"
                  className="h-10 w-full rounded-xl border border-border/80 bg-muted/20 px-3.5 text-sm text-foreground placeholder:text-muted-foreground focus:border-ring focus:outline-none focus:ring-2 focus:ring-ring/20"
                />
              </div>
            </div>

            <p className="text-[11px] text-muted-foreground">
              Enter the exact code received on your confirmation card or SMS receipt.
            </p>

            {/* Error Banner */}
            {lookupError && (
              <div className="rounded-xl border border-destructive/30 bg-destructive/5 p-3.5 text-xs text-destructive flex items-start gap-2.5">
                <AlertCircle className="size-4 shrink-0 mt-0.5" />
                <span>{lookupError}</span>
              </div>
            )}

            <Button
              disabled={isLoading || !lookupCode.trim() || !guestName.trim()}
              onClick={() => handleLookup()}
              className="h-10 w-full text-xs font-medium shadow-xs"
            >
              <Search className="size-3.5 mr-2" />
              {isLoading ? "Searching records..." : "Look up Details"}
            </Button>
          </div>
        </div>

        {/* ========================================================= */}
        {/* 2. RESULTS CARD                                           */}
        {/* ========================================================= */}
        {lookupResult && (
          <motion.div
            initial={shouldReduceMotion ? { opacity: 1 } : { opacity: 0, y: 10 }}
            animate={{ opacity: 1, y: 0 }}
            className="rounded-2xl border border-border/80 bg-card p-6 shadow-xs space-y-6"
          >
            {/* BOOKING RESULT */}
            {lookupResult.type === "booking" && (
              <div className="space-y-5">
                <div className="flex items-center justify-between pb-3 border-b border-border/70">
                  <div className="flex items-center gap-2">
                    <CalendarCheck className="size-5 text-primary" />
                    <h2 className="text-base font-semibold text-foreground">
                      Dining Reservation
                    </h2>
                  </div>
                  <StatusBadge
                    status={lookupResult.data.status === "confirmed" ? "success" : "neutral"}
                    label={lookupResult.data.status === "confirmed" ? "Confirmed" : "Cancelled"}
                  />
                </div>

                <div className="space-y-1">
                  <span className="text-xs text-muted-foreground">Confirmation Code</span>
                  <CodeBlock code={lookupResult.data.code} label="Booking Code" />
                </div>

                <div className="grid grid-cols-2 gap-4 rounded-xl border border-border/80 bg-muted/20 p-4 text-xs">
                  <div>
                    <span className="text-muted-foreground block text-[11px]">Guest Name</span>
                    <strong className="text-foreground">{lookupResult.data.guestName}</strong>
                  </div>
                  <div>
                    <span className="text-muted-foreground block text-[11px]">Party & Area</span>
                    <strong className="text-foreground">
                      {lookupResult.data.partySize} Guests · {lookupResult.data.area}
                    </strong>
                  </div>
                  <div>
                    <span className="text-muted-foreground block text-[11px]">Date & Time</span>
                    <strong className="text-foreground">
                      {lookupResult.data.date} at {lookupResult.data.time}
                    </strong>
                  </div>
                  <div>
                    <span className="text-muted-foreground block text-[11px]">Table Assigned</span>
                    <strong className="text-foreground font-mono">{lookupResult.data.tableLabel}</strong>
                  </div>
                </div>

                {/* Deposit status & refund schedule */}
                <div className="rounded-xl border border-border/70 bg-card p-3 text-xs text-muted-foreground space-y-1">
                  <strong className="text-foreground block">Deposit & Cancellation Policy:</strong>
                  <p>{lookupResult.data.cancellationPolicy}</p>
                </div>

                {/* Cancel Booking Action */}
                {lookupResult.data.status === "confirmed" && (
                  <div className="pt-2">
                    <Button
                      variant="outline"
                      onClick={handleCancelBooking}
                      className="h-10 text-xs text-destructive hover:bg-destructive/10 hover:text-destructive"
                    >
                      Cancel Booking
                    </Button>
                  </div>
                )}
              </div>
            )}

            {/* ORDER RESULT */}
            {lookupResult.type === "order" && (
              <div className="space-y-5">
                <div className="flex items-center justify-between pb-3 border-b border-border/70">
                  <div className="flex items-center gap-2">
                    <ShoppingBag className="size-5 text-primary" />
                    <h2 className="text-base font-semibold text-foreground">Pickup Order</h2>
                  </div>
                  <StatusBadge
                    status={lookupResult.data.status === "cancelled" ? "neutral" : "accent"}
                    label={
                      lookupResult.data.status === "preparing"
                        ? "Kitchen Preparing"
                        : lookupResult.data.status === "ready"
                          ? "Ready for Pickup"
                          : lookupResult.data.status === "cancelled"
                            ? "Cancelled"
                            : "Placed"
                    }
                  />
                </div>

                {/* Status Stepper Timeline */}
                <div className="space-y-2">
                  <span className="text-xs font-semibold text-foreground">Order Timeline</span>
                  <div className="grid grid-cols-4 gap-2 pt-1 text-center text-[10px]">
                    {["Placed", "Preparing", "Ready", "Completed"].map((s, idx) => {
                      const isComplete =
                        lookupResult.data.status === "completed" ||
                        (lookupResult.data.status === "ready" && idx <= 2) ||
                        (lookupResult.data.status === "preparing" && idx <= 1) ||
                        idx === 0
                      return (
                        <div key={s} className="space-y-1.5">
                          <div
                            className={`h-1.5 rounded-full ${
                              isComplete ? "bg-primary" : "bg-muted"
                            }`}
                          />
                          <span
                            className={
                              isComplete
                                ? "font-semibold text-foreground"
                                : "text-muted-foreground"
                            }
                          >
                            {s}
                          </span>
                        </div>
                      )
                    })}
                  </div>
                </div>

                {/* Lines summary */}
                <div className="rounded-xl border border-border/80 bg-muted/20 p-4 space-y-2 text-xs">
                  <div className="flex items-center justify-between pb-2 border-b border-border/60">
                    <span className="text-muted-foreground">Estimated Ready Time</span>
                    <strong className="text-foreground font-mono">{lookupResult.data.pickupTime}</strong>
                  </div>
                  {lookupResult.data.lines.map((l: any, i: number) => (
                    <div key={i} className="flex justify-between">
                      <span>{l.quantity}x {l.name}</span>
                      <Money amountMinor={l.priceMinor * l.quantity} currency="INR" />
                    </div>
                  ))}
                  <div className="flex justify-between border-t border-border/60 pt-2 font-bold text-sm">
                    <span>Total Due at Pickup:</span>
                    <Money amountMinor={lookupResult.data.totalMinor} currency="INR" />
                  </div>
                </div>

                {lookupResult.data.status !== "cancelled" && (
                  <Button
                    variant="outline"
                    onClick={handleCancelOrder}
                    className="h-10 text-xs text-destructive hover:bg-destructive/10 hover:text-destructive"
                  >
                    Cancel Order
                  </Button>
                )}
              </div>
            )}

            {/* WAITLIST RESULT */}
            {lookupResult.type === "waitlist" && (
              <div className="space-y-5">
                <div className="flex items-center justify-between pb-3 border-b border-border/70">
                  <div className="flex items-center gap-2">
                    <Users className="size-5 text-primary" />
                    <h2 className="text-base font-semibold text-foreground">Waitlist Status</h2>
                  </div>
                  <StatusBadge status="warning" label="Waiting for Table" />
                </div>

                <div className="grid grid-cols-2 gap-3 text-xs rounded-xl border border-border/80 bg-muted/20 p-4">
                  <div>
                    <span className="text-muted-foreground block text-[11px]">Position in Queue</span>
                    <strong className="text-base font-bold text-primary">
                      #{lookupResult.data.position}
                    </strong>
                  </div>
                  <div>
                    <span className="text-muted-foreground block text-[11px]">Est. Wait Time</span>
                    <strong className="text-foreground">~{lookupResult.data.estimatedWaitMinutes} mins</strong>
                  </div>
                </div>

                {/* Active Table Offer */}
                {lookupResult.data.offerActive && (
                  <div className="rounded-xl border border-primary/30 bg-primary/5 p-4 space-y-3 text-xs">
                    <div className="flex items-center justify-between">
                      <span className="font-semibold text-primary">Table Offer Ready!</span>
                      <CountdownChip expiresAt={lookupResult.data.offerExpiresAt} label="Expires in" />
                    </div>
                    <p className="text-muted-foreground leading-relaxed">
                      An indoor table for {lookupResult.data.partySize} guests has just opened up. Claim your table before the countdown expires.
                    </p>
                    <div className="flex gap-2.5 pt-1">
                      <Button
                        size="sm"
                        onClick={handleAcceptOffer}
                        className="h-9 px-4 text-xs font-medium"
                      >
                        Accept Table Offer
                      </Button>
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={handleLeaveWaitlist}
                        className="h-9 px-3 text-xs text-destructive hover:bg-destructive/10"
                      >
                        Decline & Leave
                      </Button>
                    </div>
                  </div>
                )}
              </div>
            )}
          </motion.div>
        )}
      </div>

      {/* Confirmation Dialog for Destructive Actions */}
      <AnimatePresence>
        {confirmDialog && confirmDialog.open && (
          <div
            role="dialog"
            aria-modal="true"
            aria-labelledby="manage-confirm-title"
            className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-background/80 backdrop-blur-xs"
          >
            <motion.div
              initial={shouldReduceMotion ? { opacity: 1 } : { opacity: 0, scale: 0.95 }}
              animate={{ opacity: 1, scale: 1 }}
              exit={shouldReduceMotion ? { opacity: 0 } : { opacity: 0, scale: 0.95 }}
              className="w-full max-w-md rounded-2xl border border-border bg-card p-6 shadow-2xl space-y-4"
            >
              <div className="flex items-center gap-3 text-destructive">
                <div className="flex size-10 items-center justify-center rounded-xl bg-destructive/10">
                  <ShieldAlert className="size-5" />
                </div>
                <h3 id="manage-confirm-title" className="text-base font-semibold text-foreground">
                  {confirmDialog.title}
                </h3>
              </div>

              <p className="text-xs sm:text-sm text-muted-foreground leading-relaxed">
                {confirmDialog.description}
              </p>

              <div className="flex items-center justify-end gap-2.5 pt-2">
                <Button
                  variant="outline"
                  onClick={() => setConfirmDialog(null)}
                  className="h-9 px-4 text-xs"
                >
                  Keep Active
                </Button>
                <Button
                  variant="destructive"
                  onClick={confirmDialog.onConfirm}
                  className="h-9 px-4 text-xs shadow-xs"
                >
                  {confirmDialog.confirmLabel}
                </Button>
              </div>
            </motion.div>
          </div>
        )}
      </AnimatePresence>
    </div>
  )
}
