import { useState, useEffect, lazy, Suspense } from "react"
import { motion } from "framer-motion"
import { UtensilsCrossed, Moon, Sun, Radio, Sparkles, LayoutGrid } from "lucide-react"
import { Button } from "@/components/ui/button"
import CheckoutPage from "@/pages/CheckoutPage"

const CardGallery = lazy(() => import("@/dev/CardGallery"))

function usePathname() {
  const [pathname, setPathname] = useState(window.location.pathname)
  useEffect(() => {
    const onPop = () => setPathname(window.location.pathname)
    window.addEventListener("popstate", onPop)
    return () => window.removeEventListener("popstate", onPop)
  }, [])
  return pathname
}

export default function App() {
  const pathname = usePathname()
  if (pathname === "/dev/cards" || pathname.startsWith("/dev/cards")) {
    return (
      <Suspense fallback={<div className="min-h-screen flex items-center justify-center p-6 text-sm text-muted-foreground">Loading Card Gallery...</div>}>
        <CardGallery />
      </Suspense>
    )
  }
  const checkoutMatch = pathname.match(/^\/checkout\/(.+)$/)
  if (checkoutMatch) {
    return <CheckoutPage token={decodeURIComponent(checkoutMatch[1])} />
  }
  return <LandingPage />
}

function LandingPage() {
  const [isDark, setIsDark] = useState(false)

  useEffect(() => {
    const prefersDark = window.matchMedia("(prefers-color-scheme: dark)").matches
    setIsDark(prefersDark)
    if (prefersDark) {
      document.documentElement.classList.add("dark")
    }
  }, [])

  const toggleTheme = () => {
    setIsDark((prev) => {
      const next = !prev
      if (next) {
        document.documentElement.classList.add("dark")
      } else {
        document.documentElement.classList.remove("dark")
      }
      return next
    })
  }

  return (
    <div className="relative min-h-screen flex flex-col items-center justify-center p-6 overflow-hidden bg-background text-foreground transition-colors duration-300">
      {/* Background decorative ambient glow */}
      <div className="pointer-events-none absolute inset-0 flex items-center justify-center overflow-hidden">
        <div className="h-96 w-96 rounded-full bg-primary/5 blur-3xl" />
        <div className="h-80 w-80 -translate-y-24 translate-x-32 rounded-full bg-accent/30 blur-2xl" />
      </div>

      {/* Top action bar */}
      <header className="absolute top-6 right-6 z-10 flex items-center gap-2">
        <Button
          variant="outline"
          size="sm"
          onClick={() => {
            window.history.pushState({}, "", "/dev/cards")
            window.dispatchEvent(new Event("popstate"))
          }}
          aria-label="View Card Gallery"
          className="cursor-pointer gap-2"
        >
          <LayoutGrid className="h-4 w-4" />
          <span>Card Gallery</span>
        </Button>
        <Button
          variant="outline"
          size="sm"
          onClick={toggleTheme}
          aria-label="Toggle theme"
          className="cursor-pointer gap-2"
        >
          {isDark ? <Sun className="h-4 w-4" /> : <Moon className="h-4 w-4" />}
          <span>{isDark ? "Light" : "Dark"}</span>
        </Button>
      </header>

      {/* Main card */}
      <main className="relative z-10 max-w-xl w-full text-center">
        <motion.div
          initial={{ opacity: 0, y: 16 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.5, ease: "easeOut" }}
          className="flex flex-col items-center space-y-6"
        >
          {/* Badge */}
          <div className="inline-flex items-center gap-2 px-3 py-1 rounded-full border border-border bg-card/60 backdrop-blur-sm text-xs font-medium text-muted-foreground shadow-xs">
            <Radio className="h-3.5 w-3.5 text-emerald-500 animate-pulse" />
            <span>MCP Server & Voice Simulation</span>
          </div>

          {/* Icon */}
          <div className="h-16 w-16 rounded-2xl border border-border bg-card flex items-center justify-center shadow-sm">
            <UtensilsCrossed className="h-8 w-8 text-foreground" />
          </div>

          {/* Title & Subtitle */}
          <div className="space-y-3">
            <h1 className="text-4xl sm:text-5xl font-bold tracking-tight text-foreground">
              TableTalk
            </h1>
            <p className="text-base sm:text-lg text-muted-foreground max-w-md mx-auto leading-relaxed">
              Voice-driven restaurant intelligence powered by the Model Context
              Protocol. Search menus, verify availability, and reserve tables seamlessly.
            </p>
          </div>

          {/* Status pill & feature highlights */}
          <div className="pt-2 flex flex-wrap items-center justify-center gap-3 text-xs text-muted-foreground">
            <div className="flex items-center gap-1.5 px-3 py-1.5 rounded-md border border-border bg-card">
              <Sparkles className="h-3.5 w-3.5 text-amber-500" />
              <span>Phase 1 Scaffolding</span>
            </div>
            <div className="flex items-center gap-1.5 px-3 py-1.5 rounded-md border border-border bg-card font-mono">
              <span className="h-1.5 w-1.5 rounded-full bg-emerald-500" />
              <span>GET /health : 200 OK</span>
            </div>
          </div>
        </motion.div>
      </main>

      {/* Footer */}
      <footer className="absolute bottom-6 text-xs text-muted-foreground">
        Open Source under MIT License
      </footer>
    </div>
  )
}
