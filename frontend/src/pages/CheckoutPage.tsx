import { useEffect, useState } from "react"
import { motion, AnimatePresence, useReducedMotion } from "framer-motion"
import {
  Lock,
  CheckCircle2,
  AlertCircle,
  ShieldCheck,
  Calendar,
  Users,
  ArrowLeft,
  Sparkles,
  RefreshCw,
  Clock,
} from "lucide-react"
import { Link } from "react-router-dom"
import { Button } from "@/components/ui/button"
import { Money } from "@/components/primitives/Money"
import { CountdownChip } from "@/components/primitives/CountdownChip"
import { SimulatedBanner } from "@/components/primitives/SimulatedBanner"
import { CodeBlock } from "@/components/primitives/CodeBlock"
import { Skeleton } from "@/components/primitives/Skeleton"
import { ThemeToggle } from "@/components/shell/ThemeToggle"

const BACKEND_URL =
  (import.meta.env.VITE_BACKEND_URL as string | undefined) ?? "http://localhost:8080"

export type SessionInfo = {
  status: string
  simulated?: boolean
  restaurant_name?: string
  amount?: { amount_minor: number; currency: string; display: string }
  deadline_local?: string
  party_size?: number
  start_at_local?: string
  message?: string
}

type Outcome = "success" | "decline" | "needs_verification"

interface CompleteResponse {
  status: string
  simulated: boolean
  message: string
  booking_confirmed?: boolean
}

export default function CheckoutPage({ token }: { token: string }) {
  const shouldReduceMotion = useReducedMotion()
  const [session, setSession] = useState<SessionInfo | null>(null)
  const [loading, setLoading] = useState(true)
  const [busy, setBusy] = useState(false)
  const [outcomeResult, setOutcomeResult] = useState<CompleteResponse | null>(null)
  const [isExpired, setIsExpired] = useState(false)

  // Fetch session data
  useEffect(() => {
    let cancelled = false
    setLoading(true)

    fetch(`${BACKEND_URL}/mock-checkout/${token}`)
      .then((r) => {
        if (!r.ok) {
          throw new Error("Not found or expired")
        }
        return r.json()
      })
      .then((data: SessionInfo) => {
        if (cancelled) return
        setSession(data)
        // Check if deadline already passed
        if (data.deadline_local) {
          const deadlineTime = new Date(data.deadline_local).getTime()
          if (Date.now() >= deadlineTime) {
            setIsExpired(true)
          }
        }
      })
      .catch(() => {
        if (!cancelled) {
          setSession({ status: "not_found" })
        }
      })
      .finally(() => {
        if (!cancelled) {
          setLoading(false)
        }
      })

    return () => {
      cancelled = true
    }
  }, [token])

  // Submit simulated payment outcome
  const handleSimulate = async (outcome: Outcome) => {
    if (busy || isExpired) return
    setBusy(true)

    try {
      const res = await fetch(`${BACKEND_URL}/mock-checkout/${token}/complete`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ outcome }),
      })

      const data: CompleteResponse = await res.json()
      setOutcomeResult(data)

      if (data.status === "expired") {
        setIsExpired(true)
      }
    } catch {
      setOutcomeResult({
        status: "error",
        simulated: true,
        message: "Network error occurred while connecting to simulated payment processor.",
      })
    } finally {
      setBusy(false)
    }
  }

  // Format booking date/time
  const formattedDateTime = session?.start_at_local
    ? new Date(session.start_at_local).toLocaleString([], {
        weekday: "short",
        month: "short",
        day: "numeric",
        hour: "numeric",
        minute: "2-digit",
      })
    : "Upcoming Dining Reservation"

  // Extract confirmation code from success message if available
  const confirmationCodeMatch = outcomeResult?.message?.match(/Confirmation code:\s*([A-Z0-9-]+)/i)
  const confirmationCode = confirmationCodeMatch ? confirmationCodeMatch[1] : "CONFIRMED"

  return (
    <div className="relative min-h-screen flex flex-col items-center justify-between bg-background p-4 sm:p-6 text-foreground antialiased selection:bg-primary/20 selection:text-foreground">
      {/* Top Bar: Brand + Theme Toggle */}
      <header className="w-full max-w-md flex items-center justify-between pt-2 pb-6">
        <Link
          to="/"
          className="text-base font-semibold tracking-tight text-foreground hover:opacity-85 transition-opacity"
        >
          TableTalk
        </Link>
        <ThemeToggle />
      </header>

      {/* Main Checkout Card */}
      <main className="w-full max-w-[440px] my-auto">
        <motion.div
          initial={shouldReduceMotion ? { opacity: 1 } : { opacity: 0, y: 12 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.2 }}
          className="rounded-2xl border border-border/80 bg-card p-6 sm:p-7 shadow-xs space-y-6"
        >
          {/* ========================================================= */}
          {/* 1. LOADING STATE                                          */}
          {/* ========================================================= */}
          {loading ? (
            <div className="space-y-5" aria-busy="true" aria-label="Loading payment session">
              <div className="space-y-2 text-center">
                <Skeleton className="h-6 w-32 mx-auto rounded-md" />
                <Skeleton className="h-4 w-48 mx-auto rounded-md" />
              </div>
              <Skeleton className="h-28 w-full rounded-xl" />
              <div className="space-y-2.5 pt-2">
                <Skeleton className="h-10 w-full rounded-xl" />
                <Skeleton className="h-10 w-full rounded-xl" />
                <Skeleton className="h-10 w-full rounded-xl" />
              </div>
            </div>
          ) : outcomeResult?.status === "success" || outcomeResult?.message?.includes("already completed") ? (
            /* ========================================================= */
            /* 2. SUCCESS STATE                                          */
            /* ========================================================= */
            <motion.div
              initial={shouldReduceMotion ? { opacity: 1 } : { opacity: 0, scale: 0.96 }}
              animate={{ opacity: 1, scale: 1 }}
              className="space-y-5 text-center"
            >
              <div className="mx-auto flex size-12 items-center justify-center rounded-full bg-emerald-500/10 text-emerald-600 dark:text-emerald-400">
                <CheckCircle2 className="size-7" />
              </div>

              <div className="space-y-1.5">
                <h1 className="text-xl font-semibold tracking-tight text-foreground">
                  Deposit Confirmed
                </h1>
                <p className="text-xs text-muted-foreground">
                  Your table is secured at {session?.restaurant_name ?? "The Saffron Nebula Bistro"}.
                </p>
              </div>

              {/* Confirmation Code Block */}
              <div className="space-y-1.5 text-left rounded-xl border border-border/80 bg-muted/20 p-4">
                <span className="text-[11px] font-semibold uppercase tracking-wider text-muted-foreground">
                  Confirmation Code
                </span>
                <CodeBlock code={confirmationCode} />
              </div>

              {/* Summary note */}
              <div className="rounded-xl border border-emerald-500/20 bg-emerald-500/5 p-3 text-xs text-emerald-800 dark:text-emerald-300 text-left flex items-start gap-2">
                <Sparkles className="size-4 shrink-0 mt-0.5" />
                <p>
                  No real money was charged. Your booking status has been updated in the assistant.
                </p>
              </div>

              <div className="pt-2">
                <Button asChild className="w-full h-11 text-xs font-medium shadow-xs">
                  <Link to="/assistant">
                    <ArrowLeft className="size-4 mr-2" />
                    Back to Assistant
                  </Link>
                </Button>
              </div>
            </motion.div>
          ) : isExpired || outcomeResult?.status === "expired" ? (
            /* ========================================================= */
            /* 3. EXPIRED STATE                                          */
            /* ========================================================= */
            <div className="space-y-5 text-center">
              <div className="mx-auto flex size-12 items-center justify-center rounded-full bg-muted text-muted-foreground">
                <Clock className="size-6" />
              </div>

              <div className="space-y-1.5">
                <h1 className="text-xl font-semibold tracking-tight text-foreground">
                  Hold Period Expired
                </h1>
                <p className="text-xs sm:text-sm text-muted-foreground leading-relaxed">
                  The payment deadline passed and the temporary hold on your table was released.
                </p>
              </div>

              <div className="pt-2">
                <Button asChild className="w-full h-11 text-xs font-medium">
                  <Link to="/assistant">
                    <ArrowLeft className="size-4 mr-2" />
                    Return to Assistant
                  </Link>
                </Button>
              </div>
            </div>
          ) : session?.status !== "open" ? (
            /* ========================================================= */
            /* 4. GENERIC NOT FOUND STATE                                */
            /* ========================================================= */
            <div className="space-y-5 text-center">
              <div className="mx-auto flex size-12 items-center justify-center rounded-full bg-muted text-muted-foreground">
                <Lock className="size-6" />
              </div>

              <div className="space-y-1.5">
                <h1 className="text-xl font-semibold tracking-tight text-foreground">
                  Link No Longer Valid
                </h1>
                <p className="text-xs sm:text-sm text-muted-foreground leading-relaxed">
                  This payment link is no longer valid. It may have been completed, expired, or replaced. Ask the assistant to refresh your payment link.
                </p>
              </div>

              <div className="pt-2">
                <Button asChild className="w-full h-11 text-xs font-medium">
                  <Link to="/assistant">
                    <ArrowLeft className="size-4 mr-2" />
                    Back to Assistant
                  </Link>
                </Button>
              </div>
            </div>
          ) : (
            /* ========================================================= */
            /* 5. ACTIVE PAYMENT FORM (OUTCOME SIMULATOR ONLY)           */
            /* ========================================================= */
            <>
              {/* Header: Restaurant name & Title */}
              <div className="space-y-1 text-center">
                <p className="text-xs font-medium text-muted-foreground uppercase tracking-wider">
                  {session.restaurant_name ?? "The Saffron Nebula Bistro"}
                </p>
                <h1 className="text-2xl font-semibold tracking-tight text-foreground">
                  Table Deposit Hold
                </h1>
              </div>

              {/* Simulated Banner */}
              <SimulatedBanner
                type="payment"
                message="SIMULATED PAYMENT — no real money moves"
              />

              {/* Order Summary Box */}
              <div className="rounded-xl border border-border/80 bg-muted/20 p-4 space-y-3.5">
                <div className="flex items-center justify-between">
                  <span className="text-xs text-muted-foreground">Deposit Required</span>
                  <div className="text-xl font-semibold text-foreground">
                    {session.amount ? (
                      <Money
                        amountMinor={session.amount.amount_minor}
                        currency={session.amount.currency}
                      />
                    ) : (
                      "₹1,000"
                    )}
                  </div>
                </div>

                <div className="border-t border-border/60 pt-3 space-y-2 text-xs">
                  <div className="flex items-center justify-between text-muted-foreground">
                    <span className="flex items-center gap-1.5">
                      <Calendar className="size-3.5" />
                      Reservation
                    </span>
                    <span className="font-medium text-foreground">{formattedDateTime}</span>
                  </div>

                  <div className="flex items-center justify-between text-muted-foreground">
                    <span className="flex items-center gap-1.5">
                      <Users className="size-3.5" />
                      Party Size
                    </span>
                    <span className="font-medium text-foreground">
                      {session.party_size ? `${session.party_size} Guests` : "Party reservation"}
                    </span>
                  </div>
                </div>

                {/* Deadline Countdown Chip */}
                {session.deadline_local && (
                  <div className="pt-1 flex items-center justify-between border-t border-border/60 text-xs">
                    <span className="text-muted-foreground">Hold Deadline:</span>
                    <CountdownChip
                      expiresAt={session.deadline_local}
                      onExpire={() => setIsExpired(true)}
                    />
                  </div>
                )}
              </div>

              {/* Status Outcome Message if Declined or Needs Verification */}
              <AnimatePresence>
                {outcomeResult && outcomeResult.status !== "success" && (
                  <motion.div
                    initial={shouldReduceMotion ? { opacity: 1 } : { opacity: 0, y: -4 }}
                    animate={{ opacity: 1, y: 0 }}
                    exit={{ opacity: 0 }}
                    className={`rounded-xl border p-3.5 text-xs flex items-start gap-2.5 ${
                      outcomeResult.status === "declined" || outcomeResult.status === "attempts_exhausted"
                        ? "border-destructive/30 bg-destructive/5 text-destructive"
                        : "border-amber-500/30 bg-amber-500/5 text-amber-800 dark:text-amber-300"
                    }`}
                  >
                    <AlertCircle className="size-4 shrink-0 mt-0.5" />
                    <div className="space-y-0.5">
                      <p className="font-medium">
                        {outcomeResult.status === "declined"
                          ? "Payment Declined"
                          : outcomeResult.status === "needs_verification"
                          ? "Step-up Verification"
                          : "Processor Notice"}
                      </p>
                      <p className="text-[11px] leading-relaxed opacity-90">
                        {outcomeResult.message}
                      </p>
                    </div>
                  </motion.div>
                )}
              </AnimatePresence>

              {/* Simulated Outcome Action Buttons */}
              <div className="space-y-2.5 pt-1">
                <Button
                  disabled={busy}
                  onClick={() => handleSimulate("success")}
                  className="w-full h-11 text-xs font-medium shadow-xs gap-2"
                >
                  <CheckCircle2 className="size-4 text-primary-foreground" />
                  <span>Simulate success</span>
                </Button>

                <Button
                  variant="outline"
                  disabled={busy}
                  onClick={() => handleSimulate("needs_verification")}
                  className="w-full h-10 text-xs font-medium gap-2 text-foreground"
                >
                  <ShieldCheck className="size-4 text-muted-foreground" />
                  <span>Simulate extra verification</span>
                </Button>

                <Button
                  variant="outline"
                  disabled={busy}
                  onClick={() => handleSimulate("decline")}
                  className="w-full h-10 text-xs font-medium gap-2 text-muted-foreground hover:text-destructive"
                >
                  <RefreshCw className="size-4" />
                  <span>Simulate decline</span>
                </Button>
              </div>

              {/* Trust footer note */}
              <p className="text-center text-[11px] text-muted-foreground leading-relaxed pt-2">
                No real credit card numbers or banking fields are ever requested or stored. This simulated checkout is part of the TableTalk demonstration.
              </p>
            </>
          )}
        </motion.div>
      </main>

      {/* Footer Note */}
      <footer className="w-full max-w-md py-4 text-center text-[11px] text-muted-foreground">
        Demo project. No real payments are processed.
      </footer>
    </div>
  )
}
