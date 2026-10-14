import { useState } from "react"
import { Volume2, ChevronDown, ChevronUp } from "lucide-react"

interface SpokenCaptionProps {
  text?: string | null
  defaultExpanded?: boolean
}

/**
 * SpokenCaption renders the server's spoken text as a visually secondary,
 * collapsible caption block so the transcript and the card never disagree.
 */
export function SpokenCaption({ text, defaultExpanded = true }: SpokenCaptionProps) {
  const [isExpanded, setIsExpanded] = useState(defaultExpanded)

  if (!text || !text.trim()) {
    return null
  }

  return (
    <aside
      aria-label="Spoken assistant response"
      className="mt-3 rounded-lg border border-border/60 bg-muted/30 p-2.5 text-xs text-muted-foreground transition-colors"
    >
      <button
        type="button"
        onClick={() => setIsExpanded((prev) => !prev)}
        aria-expanded={isExpanded}
        className="flex w-full cursor-pointer items-center justify-between text-left font-medium text-foreground/80 hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring rounded-sm py-0.5"
      >
        <span className="flex items-center gap-1.5">
          <Volume2 className="h-3.5 w-3.5 text-primary" aria-hidden="true" />
          <span>Spoken Response</span>
        </span>
        <span className="flex items-center gap-1 text-[11px] text-muted-foreground">
          <span>{isExpanded ? "Hide" : "Show"}</span>
          {isExpanded ? (
            <ChevronUp className="h-3.5 w-3.5" aria-hidden="true" />
          ) : (
            <ChevronDown className="h-3.5 w-3.5" aria-hidden="true" />
          )}
        </span>
      </button>

      {isExpanded && (
        <p className="mt-1.5 text-xs leading-relaxed text-foreground/90 font-normal select-text">
          "{text.trim()}"
        </p>
      )}
    </aside>
  )
}
