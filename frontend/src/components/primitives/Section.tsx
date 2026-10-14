import type { ReactNode } from "react"
import { cn } from "@/lib/utils"

export interface SectionProps {
  title?: string
  description?: string
  action?: ReactNode
  children: ReactNode
  className?: string
  containerClassName?: string
}

export function Section({
  title,
  description,
  action,
  children,
  className,
  containerClassName,
}: SectionProps) {
  return (
    <section className={cn("space-y-4", className)}>
      {(title || description || action) && (
        <div className="flex items-center justify-between gap-4 pb-2">
          <div className="space-y-0.5">
            {title && (
              <h2 className="text-lg font-semibold tracking-tight text-foreground">
                {title}
              </h2>
            )}
            {description && (
              <p className="text-xs sm:text-sm text-muted-foreground">
                {description}
              </p>
            )}
          </div>
          {action && <div className="shrink-0">{action}</div>}
        </div>
      )}
      <div className={cn("rounded-xl border border-border/80 bg-card p-5 sm:p-6 shadow-xs", containerClassName)}>
        {children}
      </div>
    </section>
  )
}
