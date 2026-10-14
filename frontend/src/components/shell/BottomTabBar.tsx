import { NavLink } from "react-router-dom"
import {
  MessageSquare,
  UtensilsCrossed,
  Calendar,
  ShoppingBag,
  SlidersHorizontal,
} from "lucide-react"
import { cn } from "@/lib/utils"

const tabs = [
  { to: "/assistant", label: "Assistant", icon: MessageSquare },
  { to: "/menu", label: "Menu", icon: UtensilsCrossed },
  { to: "/reserve", label: "Reserve", icon: Calendar },
  { to: "/order", label: "Order", icon: ShoppingBag },
  { to: "/manage", label: "Manage", icon: SlidersHorizontal },
]

export function BottomTabBar() {
  return (
    <nav
      aria-label="Mobile Navigation"
      className="md:hidden fixed bottom-0 left-0 right-0 z-40 border-t border-border/80 bg-background/90 backdrop-blur-md pb-safe"
    >
      <div className="flex h-14 items-center justify-around px-2">
        {tabs.map((tab) => {
          const Icon = tab.icon
          return (
            <NavLink
              key={tab.to}
              to={tab.to}
              className={({ isActive }) =>
                cn(
                  "flex min-w-[56px] min-h-[44px] flex-col items-center justify-center gap-1 rounded-lg px-2 py-1 text-[11px] font-medium transition-colors select-none",
                  isActive
                    ? "text-primary font-semibold"
                    : "text-muted-foreground hover:text-foreground"
                )
              }
            >
              {({ isActive }) => (
                <>
                  <Icon
                    className={cn(
                      "size-5 transition-transform",
                      isActive ? "scale-110 text-primary stroke-[2.2]" : "stroke-[1.8]"
                    )}
                    aria-hidden="true"
                  />
                  <span>{tab.label}</span>
                </>
              )}
            </NavLink>
          )
        })}
      </div>
    </nav>
  )
}
