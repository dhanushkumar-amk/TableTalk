import { useState, useEffect } from "react"
import { Copy, Check, Users, Clock, RefreshCw, XCircle, Timer } from "lucide-react"
import type { JoinWaitlistOutput, GetWaitlistStatusOutput } from "../types"
import type { CardEventHandlers } from "../events"
import { SpokenCaption } from "./SpokenCaption"

interface WaitlistCardProps {
  joinOutput?: JoinWaitlistOutput | null
  statusOutput?: GetWaitlistStatusOutput | null
  spokenText?: string | null
  events?: CardEventHandlers
}

function formatCountdown(totalSeconds: number): string {
  if (totalSeconds <= 0) return "00:00"
  const mins = Math.floor(totalSeconds / 60)
  const secs = totalSeconds % 60
  return `${String(mins).padStart(2, "0")}:${String(secs).padStart(2, "0")}`
}

export function WaitlistCard({
  joinOutput,
  statusOutput,
  spokenText,
  events,
}: WaitlistCardProps) {
  const [copied, setCopied] = useState(false)
  const [showLeaveConfirm, setShowLeaveConfirm] = useState(false)

  const waitlist = statusOutput?.waitlist
  const code = joinOutput?.code || waitlist?.code || "WL-00000"
  const guestName = joinOutput?.guest_name || waitlist?.guest_name || "Guest"
  const partySize = joinOutput?.party_size || waitlist?.party_size || 2
  const area = joinOutput?.area || waitlist?.area || "any"
  const status = statusOutput?.status || joinOutput?.status || "waiting"
  const position = joinOutput?.queue_position || waitlist?.queue_position || 1

  const offer = waitlist?.offer
  const offerSeconds = offer?.seconds_remaining || 0
  const [secondsRemaining, setSecondsRemaining] = useState<number>(offerSeconds)

  useEffect(() => {
    if (offerSeconds <= 0) return
    setSecondsRemaining(offerSeconds)
    const interval = setInterval(() => {
      setSecondsRemaining((prev) => (prev <= 1 ? 0 : prev - 1))
    }, 1000)
    return () => clearInterval(interval)
  }, [offerSeconds])

  const copyCode = async () => {
    try {
      await navigator.clipboard.writeText(code)
      setCopied(true)
      events?.onCopyCode?.({ code, label: "Waitlist Code" })
      setTimeout(() => setCopied(false), 2000)
    } catch {
      setCopied(true)
      setTimeout(() => setCopied(false), 2000)
    }
  }

  const handleLeaveConfirm = () => {
    events?.onLeaveWaitlist?.({ waitlistId: code })
    setShowLeaveConfirm(false)
  }

  return (
    <article
      aria-label={`Waitlist entry: ${code}`}
      className="flex flex-col space-y-4 rounded-2xl border border-border bg-card p-5 shadow-sm transition-all select-text"
    >
      {/* Header */}
      <header className="flex flex-wrap items-center justify-between gap-2 border-b border-border/60 pb-3">
        <div>
          <h2 className="text-sm font-semibold text-foreground">Table Waitlist</h2>
          <p className="text-xs text-muted-foreground">{guestName}</p>
        </div>

        {/* Status Badge */}
        <span
          className={`inline-flex items-center rounded-full px-2.5 py-0.5 text-xs font-semibold uppercase tracking-wider ${
            status === "offered"
              ? "bg-amber-500/15 text-amber-700 dark:text-amber-300 border border-amber-500/30"
              : status === "waiting" || status === "joined"
              ? "bg-secondary text-secondary-foreground"
              : "bg-muted text-muted-foreground"
          }`}
        >
          {status}
        </span>
      </header>

      {/* Code Box */}
      <div className="flex items-center justify-between rounded-xl border border-border/80 bg-muted/40 p-3.5">
        <div>
          <div className="text-[10px] font-mono uppercase tracking-wider text-muted-foreground">
            Waitlist Code
          </div>
          <div className="font-mono text-xl font-bold tracking-wider text-foreground select-all">
            {code}
          </div>
        </div>

        <button
          type="button"
          onClick={copyCode}
          aria-label={copied ? "Code copied to clipboard" : "Copy waitlist code"}
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

      {/* Queue Position and Details */}
      <div className="rounded-xl border border-border/70 bg-accent/20 p-4 space-y-2">
        <div className="flex items-center justify-between">
          <span className="text-xs font-semibold text-foreground">Position in Queue</span>
          <span className="font-mono text-sm font-bold text-primary">
            ~{position} <span className="text-[11px] font-normal text-muted-foreground">(approximate)</span>
          </span>
        </div>
        <div className="flex items-center gap-4 text-xs text-muted-foreground pt-1">
          <div className="flex items-center gap-1.5">
            <Users className="h-3.5 w-3.5" aria-hidden="true" />
            <span>Party of {partySize}</span>
          </div>
          <div className="flex items-center gap-1.5 capitalize">
            <Clock className="h-3.5 w-3.5" aria-hidden="true" />
            <span>{area} area</span>
          </div>
        </div>
      </div>

      {/* Active Table Offer if present */}
      {offer && (
        <div className="rounded-xl border border-amber-500/40 bg-amber-500/10 p-3.5 space-y-2">
          <div className="flex items-center justify-between">
            <span className="text-xs font-bold text-amber-800 dark:text-amber-300">
              Table Available Now!
            </span>
            <div className="flex items-center gap-1 text-xs font-mono font-semibold text-amber-800 dark:text-amber-300">
              <Timer className="h-3.5 w-3.5 animate-pulse" aria-hidden="true" />
              <span>{formatCountdown(secondsRemaining)}</span>
            </div>
          </div>
          <p className="text-xs text-foreground/90">
            A table ({offer.table_label}, {offer.area}) is ready for your party.
          </p>
        </div>
      )}

      {/* Actions */}
      <div className="flex items-center justify-between gap-2 border-t border-border/60 pt-3">
        {/* Refresh status */}
        <button
          type="button"
          onClick={() => events?.onCheckStatus?.({ type: "waitlist", id: code })}
          aria-label="Refresh waitlist queue position"
          className="inline-flex min-h-[44px] cursor-pointer items-center gap-1.5 rounded-lg border border-border bg-background px-3 py-2 text-xs font-medium text-foreground hover:bg-muted focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring transition-colors"
        >
          <RefreshCw className="h-3.5 w-3.5" aria-hidden="true" />
          <span>Check Status</span>
        </button>

        {/* Leave waitlist */}
        {!showLeaveConfirm ? (
          <button
            type="button"
            onClick={() => setShowLeaveConfirm(true)}
            aria-label="Leave waitlist"
            className="inline-flex min-h-[44px] cursor-pointer items-center gap-1.5 rounded-lg border border-transparent px-3 py-2 text-xs font-medium text-muted-foreground hover:text-destructive hover:bg-destructive/10 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring transition-colors"
          >
            <XCircle className="h-3.5 w-3.5" aria-hidden="true" />
            <span>Leave Waitlist</span>
          </button>
        ) : (
          <div className="flex items-center gap-2">
            <button
              type="button"
              onClick={handleLeaveConfirm}
              className="min-h-[44px] cursor-pointer rounded-lg bg-destructive px-3 py-2 text-xs font-semibold text-destructive-foreground hover:bg-destructive/90 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring transition-colors"
            >
              Confirm Leave
            </button>
            <button
              type="button"
              onClick={() => setShowLeaveConfirm(false)}
              className="min-h-[44px] cursor-pointer rounded-lg border border-border bg-background px-2.5 py-2 text-xs font-medium text-foreground hover:bg-muted focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring transition-colors"
            >
              Cancel
            </button>
          </div>
        )}
      </div>

      {/* Secondary Transcript Caption */}
      <SpokenCaption text={spokenText} />
    </article>
  )
}
