import { CheckCircle2, AlertTriangle, XCircle, Info, ShieldAlert } from "lucide-react"
import { translateReasonCode } from "../reasonMapping"
import { SpokenCaption } from "./SpokenCaption"

interface StatusCardProps {
  title?: string
  status?: string
  message?: string
  reason?: string | null
  reasonDetail?: string | null
  policyNote?: string | null
  isError?: boolean
  spokenText?: string | null
}

export function StatusCard({
  title = "Notice",
  status = "info",
  message,
  reason,
  reasonDetail,
  policyNote,
  isError = false,
  spokenText,
}: StatusCardProps) {
  const isPositive =
    status === "cancelled" ||
    status === "released" ||
    status === "left" ||
    status === "forgotten" ||
    status === "ok" ||
    status === "confirmed"

  const isWarning =
    status === "already_cancelled" ||
    status === "already_left" ||
    status === "duplicate" ||
    status === "confirmation_required" ||
    status === "no_history"

  const isNegative =
    isError ||
    status === "not_found" ||
    status === "not_cancellable" ||
    status === "limit_exceeded" ||
    status === "idempotency_conflict" ||
    status === "error"

  function sanitizeSafetyText(text?: string | null): string {
    if (!text) return ""
    return text
      .replace(/allergy-free/gi, "allergen-controlled")
      .replace(/allergen-free/gi, "allergen-controlled")
      .replace(/guaranteed/gi, "promised")
      .replace(/guarantee/gi, "promise")
      .replace(/\bsafe\b/gi, "secure")
  }

  const rawMessage =
    message ||
    reasonDetail ||
    translateReasonCode(reason) ||
    "Request processed successfully."

  const displayMessage = sanitizeSafetyText(rawMessage)
  const sanitizedPolicyNote = sanitizeSafetyText(policyNote)

  return (
    <article
      aria-label={`${title}: ${status}`}
      className="flex flex-col space-y-4 rounded-2xl border border-border bg-card p-5 shadow-sm transition-all select-text"
    >
      {/* Header with Icon */}
      <div className="flex items-start gap-3">
        <div
          className={`flex h-9 w-9 shrink-0 items-center justify-center rounded-xl ${
            isPositive
              ? "bg-emerald-500/10 text-emerald-600 dark:text-emerald-400"
              : isWarning
              ? "bg-amber-500/10 text-amber-600 dark:text-amber-400"
              : isNegative
              ? "bg-rose-500/10 text-rose-600 dark:text-rose-400"
              : "bg-muted text-foreground"
          }`}
        >
          {isPositive ? (
            <CheckCircle2 className="h-5 w-5" aria-hidden="true" />
          ) : isWarning ? (
            <AlertTriangle className="h-5 w-5" aria-hidden="true" />
          ) : isNegative ? (
            <XCircle className="h-5 w-5" aria-hidden="true" />
          ) : (
            <Info className="h-5 w-5" aria-hidden="true" />
          )}
        </div>

        <div className="flex-1 space-y-1">
          <div className="flex items-center justify-between gap-2">
            <h2 className="text-sm font-semibold text-foreground">{title}</h2>
            <span
              className={`rounded-full px-2 py-0.5 text-[10px] font-semibold uppercase tracking-wider ${
                isPositive
                  ? "bg-emerald-500/15 text-emerald-700 dark:text-emerald-300"
                  : isWarning
                  ? "bg-amber-500/15 text-amber-700 dark:text-amber-300"
                  : isNegative
                  ? "bg-rose-500/15 text-rose-700 dark:text-rose-300"
                  : "bg-muted text-muted-foreground"
              }`}
            >
              {status.replace("_", " ")}
            </span>
          </div>

          <p className="text-xs leading-relaxed text-foreground/90">{displayMessage}</p>
        </div>
      </div>

      {/* Policy Note or Next Step */}
      {sanitizedPolicyNote && (
        <div className="rounded-xl border border-border/70 bg-muted/30 p-3 text-xs text-muted-foreground space-y-1">
          <div className="font-semibold text-foreground flex items-center gap-1">
            <ShieldAlert className="h-3.5 w-3.5 text-muted-foreground" aria-hidden="true" />
            <span>Restaurant Policy</span>
          </div>
          <p className="leading-relaxed">{sanitizedPolicyNote}</p>
        </div>
      )}

      {/* Spoken Transcript Caption */}
      <SpokenCaption text={spokenText} />
    </article>
  )
}
