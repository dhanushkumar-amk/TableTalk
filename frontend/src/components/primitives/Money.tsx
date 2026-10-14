import { cn } from "@/lib/utils"

export interface MoneyProps {
  amountMinor?: number | null
  display?: string | null
  currency?: string
  className?: string
  size?: "sm" | "base" | "lg" | "xl"
}

export function Money({
  amountMinor,
  display,
  currency = "INR",
  className,
  size = "base",
}: MoneyProps) {
  let formatted = display ?? ""
  let accessibleLabel = ""

  if (amountMinor !== undefined && amountMinor !== null) {
    const major = amountMinor / 100
    if (!formatted) {
      if (currency === "INR") {
        formatted = `₹${major.toLocaleString("en-IN")}`
      } else {
        formatted = `${currency} ${major.toLocaleString()}`
      }
    }
    accessibleLabel = `${major.toLocaleString()} ${currency === "INR" ? "Rupees" : currency}`
  } else if (display) {
    accessibleLabel = display
  }

  const sizeClasses = {
    sm: "text-xs font-medium",
    base: "text-sm font-semibold",
    lg: "text-base font-semibold",
    xl: "text-xl font-bold tracking-tight",
  }

  return (
    <span
      className={cn("tabular-nums text-foreground", sizeClasses[size], className)}
      aria-label={accessibleLabel}
    >
      {formatted}
    </span>
  )
}
