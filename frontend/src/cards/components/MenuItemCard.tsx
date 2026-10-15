import { Flame, Sparkles, AlertCircle } from "lucide-react"
import type { MenuItemCard as MenuItemData } from "../types"
import { DishArtwork } from "@/components/menu/DishArtwork"

interface MenuItemCardProps {
  item: MenuItemData
  className?: string
}

/**
 * Visual badge for spice level (0 to 3)
 */
function SpiceLevelBadge({ level }: { level: number }) {
  if (level <= 0) return null

  return (
    <div
      className="inline-flex items-center gap-0.5 rounded-full bg-orange-500/10 px-2 py-0.5 text-[11px] font-medium text-orange-600 dark:text-orange-400"
      aria-label={`Spice level ${level} of 3`}
    >
      {Array.from({ length: Math.min(level, 3) }).map((_, i) => (
        <Flame key={i} className="h-3 w-3 fill-orange-500 text-orange-500" aria-hidden="true" />
      ))}
      <span className="ml-0.5">Spicy</span>
    </div>
  )
}

/**
 * MenuItemCard renders a single menu item with price, dietary tags, allergens,
 * availability status, chef's special badge, and match reason.
 */
export function MenuItemCard({ item, className = "" }: MenuItemCardProps) {
  const isAvailable = item.available !== false
  const priceDisplay = item.price?.display || `₹${(item.price?.amount_minor ?? 0) / 100}`

  return (
    <article
      tabIndex={0}
      aria-label={`${item.name}, ${priceDisplay}${!isAvailable ? ", unavailable" : ""}`}
      className={`group relative flex flex-col justify-between rounded-xl border border-border bg-card shadow-xs transition-all hover:shadow-md focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring select-text ${
        !isAvailable ? "opacity-75 grayscale-[30%]" : ""
      } ${className}`}
    >
      {/* Top Media / Thumbnail */}
      <div className="relative">
        <DishArtwork name={item.name} className="h-36 w-full rounded-t-xl" />

        {/* Floating Badges */}
        <div className="absolute top-2 left-2 flex flex-wrap gap-1">
          {item.chefs_special && (
            <span className="inline-flex items-center gap-1 rounded-full bg-amber-500/90 backdrop-blur-xs px-2 py-0.5 text-[10px] font-semibold text-white shadow-xs">
              <Sparkles className="h-3 w-3" aria-hidden="true" />
              <span>Chef's Special</span>
            </span>
          )}
          {!isAvailable && (
            <span className="inline-flex items-center gap-1 rounded-full bg-destructive/90 backdrop-blur-xs px-2 py-0.5 text-[10px] font-semibold text-destructive-foreground shadow-xs">
              <AlertCircle className="h-3 w-3" aria-hidden="true" />
              <span>Unavailable</span>
            </span>
          )}
        </div>

        {/* Category Pill */}
        <div className="absolute bottom-2 right-2 rounded-md bg-background/80 backdrop-blur-xs px-2 py-0.5 text-[10px] font-medium text-muted-foreground">
          {item.category}
        </div>
      </div>

      {/* Item Body */}
      <div className="flex flex-1 flex-col p-3.5 space-y-2.5">
        {/* Title and Price */}
        <div className="flex items-start justify-between gap-2">
          <h3 className="text-sm font-semibold text-foreground line-clamp-1 leading-snug">
            {item.name}
          </h3>
          <span className="shrink-0 font-mono font-bold text-sm text-foreground">
            {priceDisplay}
          </span>
        </div>

        {/* Description */}
        {item.description && (
          <p className="text-xs text-muted-foreground line-clamp-2 leading-relaxed">
            {item.description}
          </p>
        )}

        {/* Match Reason if returned */}
        {item.match_reason && item.match_reason.trim() && (
          <div className="rounded-md bg-accent/40 px-2 py-1 text-[11px] text-accent-foreground font-medium">
            💡 {item.match_reason}
          </div>
        )}

        {/* Dietary & Spice Badges */}
        <div className="flex flex-wrap items-center gap-1 pt-1">
          <SpiceLevelBadge level={item.spice_level} />
          {item.dietary_tags?.map((tag) => (
            <span
              key={tag}
              className="rounded-full bg-secondary px-2 py-0.5 text-[10px] font-medium text-secondary-foreground"
            >
              {tag.replace("_", " ")}
            </span>
          ))}
        </div>

        {/* Allergens Row (Strict Neutral Phrasing) */}
        {item.allergens && item.allergens.length > 0 && (
          <div className="mt-auto border-t border-border/50 pt-2 text-[11px] text-muted-foreground">
            <span className="font-medium text-foreground/80">Contains: </span>
            <span>{item.allergens.join(", ")}</span>
          </div>
        )}
      </div>
    </article>
  )
}
