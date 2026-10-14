import type { ComponentType } from "react"
import { CheckCircle2, AlertTriangle, AlertCircle, Clock, Info, type LucideIcon } from "lucide-react"
import { cn } from "@/lib/utils"

export type StatusVariant = "neutral" | "success" | "warning" | "danger" | "accent"

export interface StatusBadgeProps {
  status: StatusVariant
  label: string
  icon?: LucideIcon | ComponentType<{ className?: string; "aria-hidden"?: boolean | "true" | "false" }>
  size?: "sm" | "md"
  className?: string
}

const defaultIcons: Record<StatusVariant, LucideIcon> = {
  neutral: Info,
  success: CheckCircle2,
  warning: AlertTriangle,
  danger: AlertCircle,
  accent: Clock,
}

const variantStyles: Record<StatusVariant, string> = {
  neutral: "bg-muted text-foreground/80 border-border",
  success: "bg-emerald-500/10 text-emerald-800 dark:text-emerald-300 border-emerald-500/20",
  warning: "bg-amber-500/10 text-amber-800 dark:text-amber-300 border-amber-500/20",
  danger: "bg-destructive/10 text-destructive dark:text-destructive-foreground border-destructive/20",
  accent: "bg-primary/10 text-primary border-primary/25",
}

export function StatusBadge({
  status,
  label,
  icon,
  size = "md",
  className,
}: StatusBadgeProps) {
  const IconComponent = icon ?? defaultIcons[status]

  return (
    <span
      className={cn(
        "inline-flex items-center gap-1.5 font-medium rounded-full border px-2.5 py-0.5 select-none",
        size === "sm" ? "text-xs py-0.5 px-2 [&_svg]:size-3" : "text-xs py-1 px-2.5 [&_svg]:size-3.5",
        variantStyles[status],
        className
      )}
    >
      <IconComponent className="shrink-0" aria-hidden="true" />
      <span>{label}</span>
    </span>
  )
}
