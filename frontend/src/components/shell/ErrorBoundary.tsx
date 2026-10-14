import { Component, type ErrorInfo, type ReactNode } from "react"
import { AlertCircle, RotateCcw, Home } from "lucide-react"
import { Button } from "@/components/ui/button"

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
          <div className="flex size-14 items-center justify-center rounded-full bg-destructive/10 text-destructive mb-4 ring-1 ring-destructive/20">
            <AlertCircle className="size-7" aria-hidden="true" />
          </div>
          <h2 className="text-xl sm:text-2xl font-semibold tracking-tight text-foreground">
            Something went wrong
          </h2>
          <p className="mt-2 text-sm text-muted-foreground max-w-md leading-relaxed">
            An unexpected error occurred while rendering this interface. Your data has not been affected.
          </p>
          {this.state.error && (
            <pre className="mt-4 max-w-lg overflow-x-auto rounded-lg border border-border/80 bg-muted/40 p-3 text-xs font-mono text-left text-muted-foreground">
              {this.state.error.message}
            </pre>
          )}
          <div className="mt-6 flex items-center gap-3">
            <Button onClick={this.handleReset} variant="default" size="default">
              <RotateCcw className="size-4 mr-1.5" />
              Reload Page
            </Button>
            <Button asChild variant="outline" size="default">
              <a href="/assistant">
                <Home className="size-4 mr-1.5" />
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
