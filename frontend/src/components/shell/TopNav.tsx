import { NavLink } from "react-router-dom"
import { Search } from "lucide-react"
import { ThemeToggle } from "./ThemeToggle"
import { cn } from "@/lib/utils"

const navLinks = [
  { to: "/assistant", label: "Assistant" },
  { to: "/menu", label: "Menu" },
  { to: "/reserve", label: "Reserve" },
  { to: "/order", label: "Order" },
  { to: "/manage", label: "Manage" },
]

export function TopNav() {
  const triggerCommandPalette = () => {
    window.dispatchEvent(new CustomEvent("tabletalk:open-command-palette"))
  }

  return (
    <header className="sticky top-0 z-40 w-full border-b border-border/80 bg-background/85 backdrop-blur-md">
      <div className="max-w-6xl mx-auto flex h-14 items-center justify-between px-4 sm:px-6 lg:px-8">
        {/* Left: Brand Wordmark */}
        <div className="flex items-center gap-6">
          <NavLink
            to="/"
            className="flex items-center gap-1.5 font-semibold text-lg tracking-tight text-foreground hover:opacity-90 transition-opacity"
            aria-label="TableTalk Home"
          >
            <span>TableTalk</span>
            <span className="size-1.5 rounded-full bg-primary" />
          </NavLink>

          {/* Desktop Nav Links */}
          <nav className="hidden md:flex items-center gap-1" aria-label="Main Navigation">
            {navLinks.map((link) => (
              <NavLink
                key={link.to}
                to={link.to}
                className={({ isActive }) =>
                  cn(
                    "rounded-lg px-3 py-1.5 text-xs sm:text-sm font-medium transition-colors",
                    isActive
                      ? "bg-accent/80 text-foreground font-semibold"
                      : "text-muted-foreground hover:bg-muted/60 hover:text-foreground"
                  )
                }
              >
                {link.label}
              </NavLink>
            ))}
          </nav>
        </div>

        {/* Right Utility Actions */}
        <div className="flex items-center gap-2">
          {/* Command Palette Trigger */}
          <button
            type="button"
            onClick={triggerCommandPalette}
            className="flex items-center gap-2 rounded-lg border border-border/80 bg-muted/30 px-2.5 py-1.5 text-xs text-muted-foreground hover:bg-muted/70 hover:text-foreground transition-colors focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none"
            aria-label="Open Command Palette"
          >
            <Search className="size-3.5" />
            <span className="hidden sm:inline">Search</span>
            <kbd className="hidden sm:inline-flex items-center rounded border border-border bg-background px-1 text-[10px] font-mono">
              ⌘K
            </kbd>
          </button>

          {/* Privacy Link */}
          <NavLink
            to="/privacy"
            className={({ isActive }) =>
              cn(
                "hidden sm:inline-flex rounded-lg px-2.5 py-1.5 text-xs font-medium transition-colors",
                isActive
                  ? "text-primary font-semibold"
                  : "text-muted-foreground hover:text-foreground"
              )
            }
          >
            Privacy
          </NavLink>

          {/* Theme Toggle */}
          <ThemeToggle />
        </div>
      </div>
    </header>
  )
}
