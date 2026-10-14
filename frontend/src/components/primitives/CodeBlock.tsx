import { useState } from "react"
import { Copy, Check } from "lucide-react"
import { toast } from "sonner"
import { cn } from "@/lib/utils"

export interface CodeBlockProps {
  code: string
  label?: string
  className?: string
  variant?: "inline" | "card"
}

export function CodeBlock({ code, label, className, variant = "inline" }: CodeBlockProps) {
  const [copied, setCopied] = useState(false)

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(code)
      setCopied(true)
      toast.success(label ? `Copied ${label}` : "Copied to clipboard")
      setTimeout(() => setCopied(false), 2000)
    } catch {
      toast.error("Failed to copy to clipboard")
    }
  }

  if (variant === "card") {
    return (
      <div
        className={cn(
          "flex items-center justify-between gap-3 rounded-lg border border-border/80 bg-muted/40 px-3.5 py-2.5",
          className
        )}
      >
        <div className="flex flex-col">
          {label && (
            <span className="text-[10px] uppercase font-semibold tracking-wider text-muted-foreground">
              {label}
            </span>
          )}
          <span className="font-mono text-sm sm:text-base font-bold tracking-widest text-foreground select-all">
            {code}
          </span>
        </div>
        <button
          type="button"
          onClick={handleCopy}
          className="flex size-8 shrink-0 items-center justify-center rounded-md border border-border bg-background text-muted-foreground transition-colors hover:bg-muted hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none"
          aria-label={copied ? "Copied" : `Copy ${label ?? "code"}`}
        >
          {copied ? <Check className="size-4 text-emerald-600 dark:text-emerald-400" /> : <Copy className="size-4" />}
        </button>
      </div>
    )
  }

  return (
    <div
      className={cn(
        "inline-flex items-center gap-2 rounded-md border border-border/80 bg-muted/50 px-2.5 py-1 text-xs",
        className
      )}
    >
      <code className="font-mono font-semibold tracking-wider text-foreground select-all">
        {code}
      </code>
      <button
        type="button"
        onClick={handleCopy}
        className="text-muted-foreground transition-colors hover:text-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring rounded"
        aria-label={copied ? "Copied" : `Copy ${label ?? "code"}`}
      >
        {copied ? (
          <Check className="size-3 text-emerald-600 dark:text-emerald-400" />
        ) : (
          <Copy className="size-3" />
        )}
      </button>
    </div>
  )
}
