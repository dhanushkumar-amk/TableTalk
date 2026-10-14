import { Outlet, useLocation } from "react-router-dom"
import { Toaster } from "sonner"
import { TopNav } from "./TopNav"
import { BottomTabBar } from "./BottomTabBar"
import { CommandPalette } from "./CommandPalette"
import { ErrorBoundary } from "./ErrorBoundary"
import { useTheme } from "@/lib/theme"

export function AppShell() {
  const location = useLocation()
  const { resolvedTheme } = useTheme()

  // Distraction-free routes like /checkout/:token hide the main navigation bars
  const isDistractionFree = location.pathname.startsWith("/checkout")

  return (
    <div className="min-h-screen flex flex-col bg-background text-foreground antialiased selection:bg-primary/20 selection:text-foreground">
      {!isDistractionFree && <TopNav />}

      <div className="flex-1">
        <ErrorBoundary>
          <Outlet />
        </ErrorBoundary>
      </div>

      {!isDistractionFree && <BottomTabBar />}

      <CommandPalette />

      <Toaster
        theme={resolvedTheme}
        position="bottom-right"
        toastOptions={{
          className:
            "rounded-xl border border-border/80 bg-popover text-foreground shadow-lg text-xs font-medium font-sans",
        }}
        richColors
        closeButton
      />
    </div>
  )
}
