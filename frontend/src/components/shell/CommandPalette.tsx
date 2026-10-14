import { useEffect, useState } from "react"
import { useNavigate } from "react-router-dom"
import { Command } from "cmdk"
import {
  MessageSquare,
  UtensilsCrossed,
  Calendar,
  ShoppingBag,
  SlidersHorizontal,
  Shield,
  LayoutGrid,
  Sun,
  Moon,
  Home,
  Search,
} from "lucide-react"
import { useTheme } from "@/lib/theme"

export function CommandPalette() {
  const [open, setOpen] = useState(false)
  const navigate = useNavigate()
  const { setTheme } = useTheme()

  useEffect(() => {
    const down = (e: KeyboardEvent) => {
      if ((e.key === "k" && (e.metaKey || e.ctrlKey)) || (e.key === "/" && (e.target as HTMLElement)?.tagName !== "INPUT" && (e.target as HTMLElement)?.tagName !== "TEXTAREA")) {
        e.preventDefault()
        setOpen((prev) => !prev)
      }
    }

    const openListener = () => setOpen(true)
    window.addEventListener("keydown", down)
    window.addEventListener("tabletalk:open-command-palette", openListener)
    return () => {
      window.removeEventListener("keydown", down)
      window.removeEventListener("tabletalk:open-command-palette", openListener)
    }
  }, [])

  const runCommand = (command: () => void) => {
    setOpen(false)
    command()
  }

  if (!open) return null

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-label="Command Palette"
      className="fixed inset-0 z-50 flex items-start justify-center pt-[15vh] p-4 bg-background/80 backdrop-blur-xs animate-in fade-in-0 duration-150"
      onClick={() => setOpen(false)}
    >
      <div
        className="w-full max-w-lg overflow-hidden rounded-2xl border border-border/80 bg-popover shadow-2xl animate-in zoom-in-95 duration-150"
        onClick={(e) => e.stopPropagation()}
      >
        <Command className="flex flex-col w-full h-full">
          <div className="flex items-center border-b border-border/60 px-3 py-2.5">
            <Search className="size-4 shrink-0 text-muted-foreground mr-2" aria-hidden="true" />
            <Command.Input
              autoFocus
              placeholder="Type a command or search pages..."
              className="w-full bg-transparent text-sm text-foreground placeholder:text-muted-foreground outline-none"
            />
            <kbd className="hidden sm:inline-flex items-center gap-0.5 rounded border border-border bg-muted px-1.5 py-0.5 text-[10px] font-mono text-muted-foreground">
              ESC
            </kbd>
          </div>

          <Command.List className="max-h-[300px] overflow-y-auto p-2 space-y-1">
            <Command.Empty className="py-6 text-center text-xs text-muted-foreground">
              No results found.
            </Command.Empty>

            <Command.Group heading="Navigation" className="px-1.5 py-1 text-[11px] font-semibold text-muted-foreground uppercase tracking-wider">
              <Command.Item
                onSelect={() => runCommand(() => navigate("/"))}
                className="flex items-center gap-2.5 rounded-lg px-2.5 py-2 text-xs font-medium text-foreground cursor-pointer data-[selected=true]:bg-accent data-[selected=true]:text-accent-foreground select-none"
              >
                <Home className="size-4 text-muted-foreground" />
                <span>Home / Landing</span>
              </Command.Item>
              <Command.Item
                onSelect={() => runCommand(() => navigate("/assistant"))}
                className="flex items-center gap-2.5 rounded-lg px-2.5 py-2 text-xs font-medium text-foreground cursor-pointer data-[selected=true]:bg-accent data-[selected=true]:text-accent-foreground select-none"
              >
                <MessageSquare className="size-4 text-primary" />
                <span>Voice Assistant</span>
                <span className="ml-auto text-[10px] text-muted-foreground font-mono">/assistant</span>
              </Command.Item>
              <Command.Item
                onSelect={() => runCommand(() => navigate("/menu"))}
                className="flex items-center gap-2.5 rounded-lg px-2.5 py-2 text-xs font-medium text-foreground cursor-pointer data-[selected=true]:bg-accent data-[selected=true]:text-accent-foreground select-none"
              >
                <UtensilsCrossed className="size-4 text-muted-foreground" />
                <span>Browse Menu</span>
                <span className="ml-auto text-[10px] text-muted-foreground font-mono">/menu</span>
              </Command.Item>
              <Command.Item
                onSelect={() => runCommand(() => navigate("/reserve"))}
                className="flex items-center gap-2.5 rounded-lg px-2.5 py-2 text-xs font-medium text-foreground cursor-pointer data-[selected=true]:bg-accent data-[selected=true]:text-accent-foreground select-none"
              >
                <Calendar className="size-4 text-muted-foreground" />
                <span>Reserve a Table</span>
                <span className="ml-auto text-[10px] text-muted-foreground font-mono">/reserve</span>
              </Command.Item>
              <Command.Item
                onSelect={() => runCommand(() => navigate("/order"))}
                className="flex items-center gap-2.5 rounded-lg px-2.5 py-2 text-xs font-medium text-foreground cursor-pointer data-[selected=true]:bg-accent data-[selected=true]:text-accent-foreground select-none"
              >
                <ShoppingBag className="size-4 text-muted-foreground" />
                <span>Order Pickup</span>
                <span className="ml-auto text-[10px] text-muted-foreground font-mono">/order</span>
              </Command.Item>
              <Command.Item
                onSelect={() => runCommand(() => navigate("/manage"))}
                className="flex items-center gap-2.5 rounded-lg px-2.5 py-2 text-xs font-medium text-foreground cursor-pointer data-[selected=true]:bg-accent data-[selected=true]:text-accent-foreground select-none"
              >
                <SlidersHorizontal className="size-4 text-muted-foreground" />
                <span>Manage Bookings & Orders</span>
                <span className="ml-auto text-[10px] text-muted-foreground font-mono">/manage</span>
              </Command.Item>
              <Command.Item
                onSelect={() => runCommand(() => navigate("/privacy"))}
                className="flex items-center gap-2.5 rounded-lg px-2.5 py-2 text-xs font-medium text-foreground cursor-pointer data-[selected=true]:bg-accent data-[selected=true]:text-accent-foreground select-none"
              >
                <Shield className="size-4 text-muted-foreground" />
                <span>Privacy & Preferences</span>
                <span className="ml-auto text-[10px] text-muted-foreground font-mono">/privacy</span>
              </Command.Item>
              <Command.Item
                onSelect={() => runCommand(() => navigate("/dev/cards"))}
                className="flex items-center gap-2.5 rounded-lg px-2.5 py-2 text-xs font-medium text-foreground cursor-pointer data-[selected=true]:bg-accent data-[selected=true]:text-accent-foreground select-none"
              >
                <LayoutGrid className="size-4 text-muted-foreground" />
                <span>Card Gallery (Dev)</span>
                <span className="ml-auto text-[10px] text-muted-foreground font-mono">/dev/cards</span>
              </Command.Item>
            </Command.Group>

            <Command.Group heading="Preferences" className="px-1.5 py-1 text-[11px] font-semibold text-muted-foreground uppercase tracking-wider pt-2 border-t border-border/40">
              <Command.Item
                onSelect={() => runCommand(() => setTheme("light"))}
                className="flex items-center gap-2.5 rounded-lg px-2.5 py-2 text-xs font-medium text-foreground cursor-pointer data-[selected=true]:bg-accent data-[selected=true]:text-accent-foreground select-none"
              >
                <Sun className="size-4 text-muted-foreground" />
                <span>Switch to Light Theme</span>
              </Command.Item>
              <Command.Item
                onSelect={() => runCommand(() => setTheme("dark"))}
                className="flex items-center gap-2.5 rounded-lg px-2.5 py-2 text-xs font-medium text-foreground cursor-pointer data-[selected=true]:bg-accent data-[selected=true]:text-accent-foreground select-none"
              >
                <Moon className="size-4 text-muted-foreground" />
                <span>Switch to Dark Theme</span>
              </Command.Item>
            </Command.Group>
          </Command.List>
        </Command>
      </div>
    </div>
  )
}
