import { Link } from "react-router-dom"
import { MessageSquare, Home } from "lucide-react"
import { Button } from "@/components/ui/button"
import { NotFoundIllustration } from "@/components/system/SystemIllustrations"

export function NotFoundPage() {
  return (
    <div className="min-h-[70vh] flex flex-col items-center justify-center p-6 text-center">
      <NotFoundIllustration className="size-36 mb-6" />

      <p className="text-xs font-semibold uppercase tracking-wider text-primary">
        404 Error
      </p>

      <h1 className="mt-2 text-2xl sm:text-3xl font-semibold tracking-tight text-foreground">
        Page not found
      </h1>

      <p className="mt-2.5 text-xs sm:text-sm text-muted-foreground max-w-sm leading-relaxed">
        The link you followed does not exist or may have been relocated. Ask the voice assistant to help find what you need.
      </p>

      <div className="mt-6 flex flex-wrap items-center justify-center gap-3">
        <Button asChild size="default" className="shadow-xs h-10 px-5 text-xs font-medium">
          <Link to="/assistant">
            <MessageSquare className="size-4 mr-2" />
            Talk to Assistant
          </Link>
        </Button>
        <Button asChild variant="outline" size="default" className="h-10 px-5 text-xs font-medium">
          <Link to="/">
            <Home className="size-4 mr-2" />
            Go to Home
          </Link>
        </Button>
      </div>
    </div>
  )
}
