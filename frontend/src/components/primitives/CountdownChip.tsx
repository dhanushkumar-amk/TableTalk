import { useEffect, useState } from "react"
import { Clock } from "lucide-react"
import { cn } from "@/lib/utils"

export interface CountdownChipProps {
  expiresAt?: string
  secondsRemaining?: number
  onExpire?: () => void
  label?: string
  className?: string
}

export function CountdownChip({
  expiresAt,
  secondsRemaining: initialSeconds,
  onExpire,
  label = "Hold expires in",
  className,
}: CountdownChipProps) {
  const [remaining, setRemaining] = useState<number>(() => {
    if (initialSeconds !== undefined) return Math.max(0, initialSeconds)
    if (expiresAt) {
      const diff = Math.floor((new Date(expiresAt).getTime() - Date.now()) / 1000)
      return Math.max(0, diff)
    }
    return 0
  })

  useEffect(() => {
    if (remaining <= 0) {
      onExpire?.()
      return
    }

    const interval = setInterval(() => {
      setRemaining((prev) => {
        if (prev <= 1) {
          clearInterval(interval)
          onExpire?.()
          return 0
        }
        return prev - 1
      })
    }, 1000)

    return () => clearInterval(interval)
  }, [remaining, onExpire])

  const minutes = Math.floor(remaining / 60)
  const seconds = remaining % 60
  const formatted = `${minutes}:${seconds.toString().padStart(2, "0")}`

  const isUrgent = remaining > 0 && remaining < 300 // under 5 minutes
  const isExpired = remaining <= 0

  return (
    <span
      className={cn(
        "inline-flex items-center gap-1.5 rounded-full border px-2.5 py-1 text-xs font-medium tabular-nums transition-colors",
        isExpired
          ? "border-destructive/30 bg-destructive/10 text-destructive"
          : isUrgent
          ? "border-amber-500/30 bg-amber-500/10 text-amber-700 dark:text-amber-300 animate-pulse"
          : "border-border bg-muted/60 text-muted-foreground",
        className
      )}
      aria-label={`${label}: ${isExpired ? "Expired" : formatted}`}
    >
      <Clock className="size-3.5 shrink-0" aria-hidden="true" />
      <span>{label}</span>
      <span className="font-semibold text-foreground">
        {isExpired ? "Expired" : formatted}
      </span>
    </span>
  )
}
