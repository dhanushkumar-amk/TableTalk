interface IllustrationProps {
  className?: string
}

/**
 * Minimalist, elegant SVG illustrations strictly using currentColor and design system tokens.
 * Zero external assets, zero remote images, perfectly scalable and lightweight.
 */

export function NotFoundIllustration({ className = "size-32" }: IllustrationProps) {
  return (
    <svg
      viewBox="0 0 120 120"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      className={className}
      aria-hidden="true"
    >
      {/* Soft background glow circle */}
      <circle cx="60" cy="60" r="48" className="fill-primary/5 stroke-border" strokeWidth="1" />
      {/* Elegant Dining Table Silhouette */}
      <ellipse cx="60" cy="68" rx="34" ry="12" className="fill-muted/40 stroke-foreground/40" strokeWidth="1.5" />
      <path d="M40 76v16M80 76v16M60 80v16" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" className="text-foreground/30" />
      {/* Plate & Cutlery */}
      <ellipse cx="60" cy="67" rx="10" ry="4" className="stroke-primary" strokeWidth="1.5" />
      <path d="M44 64v6M76 64v6" stroke="currentColor" strokeWidth="1.2" strokeLinecap="round" className="text-foreground/40" />
      {/* Floating 404 Question Mark / Compass accent */}
      <circle cx="60" cy="38" r="12" className="fill-background stroke-primary" strokeWidth="1.5" />
      <path
        d="M57 34c.5-1.5 2-2 3.5-1.5 1.5.5 2.5 1.8 2 3.2-.8 2-3 2.5-3 4.3M60 43.5v.5"
        stroke="currentColor"
        strokeWidth="1.5"
        strokeLinecap="round"
        className="text-primary"
      />
    </svg>
  )
}

export function ErrorIllustration({ className = "size-32" }: IllustrationProps) {
  return (
    <svg
      viewBox="0 0 120 120"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      className={className}
      aria-hidden="true"
    >
      <circle cx="60" cy="60" r="48" className="fill-destructive/5 stroke-border" strokeWidth="1" />
      {/* Hexagon Shield */}
      <path
        d="M60 28l28 16v24c0 18-12 30-28 36-16-6-28-18-28-36V44l28-16z"
        className="fill-muted/30 stroke-destructive/50"
        strokeWidth="1.5"
      />
      {/* Alert mark */}
      <path d="M60 48v16M60 72v1" stroke="currentColor" strokeWidth="2" strokeLinecap="round" className="text-destructive" />
    </svg>
  )
}

export function OfflineIllustration({ className = "size-32" }: IllustrationProps) {
  return (
    <svg
      viewBox="0 0 120 120"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      className={className}
      aria-hidden="true"
    >
      <circle cx="60" cy="60" r="48" className="fill-muted/20 stroke-border" strokeWidth="1" />
      {/* Cloud silhouette */}
      <path
        d="M40 72h42a14 14 0 0 0 3-27.7 18 18 0 0 0-33-8.3A15 15 0 0 0 40 72z"
        className="fill-muted/50 stroke-foreground/40"
        strokeWidth="1.5"
      />
      {/* Slash / disconnected line */}
      <path d="M34 84l52-48" stroke="currentColor" strokeWidth="2" strokeLinecap="round" className="text-primary" />
    </svg>
  )
}

export function BackendUnreachableIllustration({ className = "size-32" }: IllustrationProps) {
  return (
    <svg
      viewBox="0 0 120 120"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      className={className}
      aria-hidden="true"
    >
      <circle cx="60" cy="60" r="48" className="fill-amber-500/5 stroke-border" strokeWidth="1" />
      {/* Server Tower silhouettes */}
      <rect x="36" y="34" width="48" height="16" rx="4" className="fill-muted/40 stroke-foreground/40" strokeWidth="1.5" />
      <circle cx="44" cy="42" r="2" className="fill-primary" />
      <circle cx="50" cy="42" r="2" className="fill-muted-foreground" />
      <line x1="60" y1="42" x2="76" y2="42" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" className="text-foreground/30" />

      <rect x="36" y="56" width="48" height="16" rx="4" className="fill-muted/40 stroke-foreground/40" strokeWidth="1.5" />
      <circle cx="44" cy="64" r="2" className="fill-primary" />
      <circle cx="50" cy="64" r="2" className="fill-muted-foreground" />
      <line x1="60" y1="64" x2="76" y2="64" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" className="text-foreground/30" />

      {/* Pulse warning */}
      <circle cx="60" cy="88" r="9" className="fill-amber-500/20 stroke-amber-500" strokeWidth="1.5" />
      <path d="M60 84v5M60 91v.5" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" className="text-amber-600 dark:text-amber-400" />
    </svg>
  )
}
