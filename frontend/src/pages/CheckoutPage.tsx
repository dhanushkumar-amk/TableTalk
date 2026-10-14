import { useEffect, useState } from "react"
import { motion } from "framer-motion"
import { ShieldAlert, CheckCircle2, XCircle, RefreshCw, Lock } from "lucide-react"
import { Button } from "@/components/ui/button"

/**
 * Minimal simulated checkout page (Phase 12).
 *
 * There is deliberately NO card form here: the mock checkout accepts only an
 * outcome selector. No real money moves. Full UI polish is Phase 13.
 */

const BACKEND_URL =
  (import.meta.env.VITE_BACKEND_URL as string | undefined) ?? "http://localhost:8080"

type SessionInfo = {
  status: string
  simulated?: boolean
  restaurant_name?: string
  amount?: { amount_minor: number; currency: string; display: string }
  deadline_local?: string
  message?: string
}

type Outcome = "success" | "decline" | "needs_verification"

export default function CheckoutPage({ token }: { token: string }) {
  const [session, setSession] = useState<SessionInfo | null>(null)
  const [loading, setLoading] = useState(true)
  const [result, setResult] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)

  useEffect(() => {
    let cancelled = false
    fetch(`${BACKEND_URL}/mock-checkout/${token}`)
      .then((r) => r.json().then((body) => ({ ok: r.ok, body })))
      .then(({ body }) => {
        if (!cancelled) setSession(body as SessionInfo)
      })
      .catch(() => {
        if (!cancelled) setSession({ status: "not_found" })
      })
      .finally(() => {
        if (!cancelled) setLoading(false)
      })
    return () => {
      cancelled = true
    }
  }, [token])

  const submit = async (outcome: Outcome) => {
    setBusy(true)
    setResult(null)
    try {
      const res = await fetch(`${BACKEND_URL}/mock-checkout/${token}/complete`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ outcome }),
      })
      const body = await res.json()
      setResult(body.status as string)
    } catch {
      setResult("error")
    } finally {
      setBusy(false)
    }
  }

  return (
    <div className="relative min-h-screen flex flex-col items-center justify-center p-6 bg-background text-foreground">
      <motion.main
        initial={{ opacity: 0, y: 16 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.4, ease: "easeOut" }}
        className="w-full max-w-md space-y-5 rounded-2xl border border-border bg-card p-6 shadow-sm"
      >
        {/* Banner */}
        <div className="flex items-center gap-2 rounded-lg border border-amber-500/40 bg-amber-500/10 px-3 py-2 text-sm font-semibold text-amber-600">
          <ShieldAlert className="h-4 w-4" />
          <span>SIMULATED PAYMENT — no real money</span>
        </div>

        <div className="space-y-1 text-center">
          <h1 className="text-2xl font-bold tracking-tight">Table Deposit</h1>
          <p className="text-sm text-muted-foreground">
            {session?.restaurant_name ?? "TableTalk"}
          </p>
        </div>

        {loading ? (
          <p className="text-center text-muted-foreground">Loading payment…</p>
        ) : session?.status !== "open" ? (
          <div className="space-y-3 text-center">
            <Lock className="mx-auto h-6 w-6 text-muted-foreground" />
            <p className="font-medium">This payment link is no longer valid.</p>
            <p className="text-sm text-muted-foreground">
              It may have been completed, expired, or replaced. Ask the assistant to
              refresh your payment link.
            </p>
          </div>
        ) : (
          <>
            <div className="rounded-xl border border-border bg-background/60 p-4 text-center">
              <div className="text-3xl font-bold">
                {session.amount?.display ?? "—"}
              </div>
              {session.deadline_local && (
                <div className="mt-1 text-xs text-muted-foreground">
                  Pay before {new Date(session.deadline_local).toLocaleString()}
                </div>
              )}
            </div>

            <div className="grid gap-2">
              <Button
                className="cursor-pointer gap-2"
                disabled={busy}
                onClick={() => submit("success")}
              >
                <CheckCircle2 className="h-4 w-4" /> Pay deposit (simulated)
              </Button>
              <Button
                variant="outline"
                className="cursor-pointer gap-2"
                disabled={busy}
                onClick={() => submit("needs_verification")}
              >
                <RefreshCw className="h-4 w-4" /> Needs extra verification
              </Button>
              <Button
                variant="outline"
                className="cursor-pointer gap-2"
                disabled={busy}
                onClick={() => submit("decline")}
              >
                <XCircle className="h-4 w-4" /> Simulate a decline
              </Button>
            </div>

            {result && <OutcomeMessage status={result} />}

            <p className="text-center text-xs text-muted-foreground">
              No card details are ever requested or stored. This is a mock ledger
              for a hackathon demo.
            </p>
          </>
        )}
      </motion.main>
    </div>
  )
}

function OutcomeMessage({ status }: { status: string }) {
  const map: Record<string, string> = {
    success: "Payment completed (simulated). Your table is confirmed!",
    declined: "The payment was declined. You can try again while the hold lasts.",
    needs_verification: "Extra verification required. Confirm again to complete.",
    expired: "The payment deadline passed and the hold was released.",
    attempts_exhausted: "This payment link has reached its retry limit.",
    rate_limited: "Too many attempts. Please wait a moment and try again.",
    not_found: "This payment link is no longer valid.",
    error: "Something went wrong. Please try again.",
  }
  return (
    <div className="rounded-lg border border-border bg-background/60 p-3 text-center text-sm">
      {map[status] ?? "Payment status updated."}
    </div>
  )
}
