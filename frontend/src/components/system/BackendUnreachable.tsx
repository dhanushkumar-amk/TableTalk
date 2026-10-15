import { RotateCcw, Home } from "lucide-react"
import { Link } from "react-router-dom"
import { Button } from "@/components/ui/button"
import { BackendUnreachableIllustration } from "@/components/system/SystemIllustrations"

interface BackendUnreachableProps {
  onRetry?: () => void
  serviceName?: string
}

export function BackendUnreachable({
  onRetry,
  serviceName = "Restaurant Server",
}: BackendUnreachableProps) {
  return (
    <div className="min-h-[50vh] flex flex-col items-center justify-center p-6 text-center">
      <BackendUnreachableIllustration className="size-32 mb-5" />

      <p className="text-xs font-semibold uppercase tracking-wider text-amber-600 dark:text-amber-400">
        Connection Interrupted
      </p>

      <h2 className="mt-2 text-xl sm:text-2xl font-semibold tracking-tight text-foreground">
        {serviceName} unreachable
      </h2>

      <p className="mt-2 text-xs sm:text-sm text-muted-foreground max-w-sm leading-relaxed">
        We could not reach the TableTalk MCP backend. Please check that your local server process is active and try again.
      </p>

      <div className="mt-6 flex flex-wrap items-center justify-center gap-3">
        {onRetry && (
          <Button onClick={onRetry} className="h-10 px-5 text-xs font-medium shadow-xs">
            <RotateCcw className="size-4 mr-2" />
            Retry Connection
          </Button>
        )}
        <Button asChild variant="outline" className="h-10 px-5 text-xs font-medium">
          <Link to="/">
            <Home className="size-4 mr-2" />
            Return Home
          </Link>
        </Button>
      </div>
    </div>
  )
}
