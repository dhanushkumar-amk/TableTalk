import { MessageSquare, RefreshCw } from "lucide-react"
import { SpokenCaption } from "./SpokenCaption"

interface FallbackCardProps {
  spokenText?: string | null
  errorReason?: string
  toolName?: string
}

/**
 * FallbackCard renders gracefully when an incoming tool response is corrupt,
 * unexpected, or does not match any registered card component.
 *
 * SAFETY INVARIANT:
 * - Shows the plain spoken assistant text.
 * - NEVER crashes.
 * - NEVER dumps raw JSON or stack traces onto the screen.
 */
export function FallbackCard({
  spokenText,
  errorReason,
  toolName,
}: FallbackCardProps) {
  return (
    <article
      aria-label="Assistant response"
      className="flex flex-col space-y-4 rounded-2xl border border-border bg-card p-5 shadow-sm transition-all select-text"
    >
      {/* Header */}
      <div className="flex items-start gap-3">
        <div className="flex h-9 w-9 shrink-0 items-center justify-center rounded-xl bg-primary/10 text-primary">
          <MessageSquare className="h-5 w-5" aria-hidden="true" />
        </div>

        <div className="flex-1 space-y-1">
          <h2 className="text-sm font-semibold text-foreground">
            TableTalk Assistant{toolName ? ` (${toolName})` : ""}
          </h2>
          <p className="text-xs text-muted-foreground">
            {spokenText || "I've processed your request. Please let me know how I can help next."}
          </p>
        </div>
      </div>

      {/* Helpful Next Step Hint */}
      <div className="flex items-center gap-2 rounded-xl bg-muted/30 p-3 text-xs text-muted-foreground border border-border/50">
        <RefreshCw className="h-3.5 w-3.5 shrink-0 text-muted-foreground" aria-hidden="true" />
        <span>
          {errorReason
            ? `Notice: Showing standard response (${errorReason}). You can ask to repeat or clarify.`
            : "You can ask for menu recommendations, check table availability, or manage your bookings."}
        </span>
      </div>

      {/* Transcript Caption */}
      <SpokenCaption text={spokenText} />
    </article>
  )
}
