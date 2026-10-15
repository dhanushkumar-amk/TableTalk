import { Component, type ErrorInfo, type ReactNode } from "react"
import { RotateCcw, Home } from "lucide-react"
import { Button } from "@/components/ui/button"
import { ErrorIllustration } from "@/components/system/SystemIllustrations"

interface Props {
  children: ReactNode
}

interface State {
  hasError: boolean
  error?: Error
}

export class ErrorBoundary extends Component<Props, State> {
  public state: State = {
    hasError: false,
  }

  public static getDerivedStateFromError(error: Error): State {
    return { hasError: true, error }
  }

  public componentDidCatch(error: Error, errorInfo: ErrorInfo) {
    console.error("Uncaught frontend error:", error, errorInfo)
  }

  private handleReset = () => {
    this.setState({ hasError: false, error: undefined })
    window.location.reload()
  }

  public render() {
    if (this.state.hasError) {
      return (
        <div className="min-h-[70vh] flex flex-col items-center justify-center p-6 text-center">
          <ErrorIllustration className="size-36 mb-6" />

          <p className="text-xs font-semibold uppercase tracking-wider text-destructive">
            Application Error
          </p>

          <h2 className="mt-2 text-2xl font-semibold tracking-tight text-foreground">
            Something unexpected occurred
          </h2>

          <p className="mt-2 text-xs sm:text-sm text-muted-foreground max-w-md leading-relaxed">
            An isolated runtime error occurred while rendering this view. Your session and dining memory remain intact.
          </p>

          {this.state.error && (
            <pre className="mt-4 max-w-md overflow-x-auto rounded-xl border border-border/80 bg-muted/40 p-3 text-[11px] font-mono text-left text-muted-foreground">
              {this.state.error.message}
            </pre>
          )}

          <div className="mt-6 flex flex-wrap items-center justify-center gap-3">
            <Button onClick={this.handleReset} variant="default" className="shadow-xs h-10 px-5 text-xs font-medium">
              <RotateCcw className="size-4 mr-2" />
              Reload Page
            </Button>
            <Button asChild variant="outline" className="h-10 px-5 text-xs font-medium">
              <a href="/assistant">
                <Home className="size-4 mr-2" />
                Return to Assistant
              </a>
            </Button>
          </div>
        </div>
      )
    }

    return this.props.children
  }
}
