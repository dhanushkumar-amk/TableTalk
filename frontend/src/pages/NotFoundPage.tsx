import { Link } from "react-router-dom"
import { MessageSquare, Home, Compass } from "lucide-react"
import { Button } from "@/components/ui/button"

export function NotFoundPage() {
  return (
    <div className="min-h-[60vh] flex flex-col items-center justify-center p-6 text-center">
      <div className="flex size-16 items-center justify-center rounded-2xl bg-muted/60 text-muted-foreground ring-1 ring-border/80 mb-5">
        <Compass className="size-8 text-primary" aria-hidden="true" />
      </div>
      <p className="text-xs font-semibold uppercase tracking-wider text-primary">
        404 Error
      </p>
      <h1 className="mt-2 text-2xl sm:text-3xl font-semibold tracking-tight text-foreground">
        Page not found
      </h1>
      <p className="mt-2 text-sm text-muted-foreground max-w-sm leading-relaxed">
        The table or page you are looking for does not exist or may have been relocated.
      </p>
      <div className="mt-6 flex items-center gap-3">
        <Button asChild variant="default">
          <Link to="/assistant">
            <MessageSquare className="size-4 mr-1.5" />
            Talk to Assistant
          </Link>
        </Button>
        <Button asChild variant="outline">
          <Link to="/">
            <Home className="size-4 mr-1.5" />
            Go Home
          </Link>
        </Button>
      </div>
    </div>
  )
}
