import { useRef } from "react"
import { ChevronLeft, ChevronRight, Utensils } from "lucide-react"
import type { SearchMenuOutput } from "../types"
import { MenuItemCard } from "./MenuItemCard"
import { AllergenNotice } from "./AllergenNotice"
import { SpokenCaption } from "./SpokenCaption"

interface MenuCarouselProps {
  structuredContent: SearchMenuOutput
  spokenText?: string | null
}

/**
 * MenuCarousel renders a responsive horizontal snap-scroll list of menu items
 * with accessible keyboard arrow controls, persistent allergen notice, and spoken caption.
 */
export function MenuCarousel({ structuredContent, spokenText }: MenuCarouselProps) {
  const scrollRef = useRef<HTMLDivElement>(null)
  const items = structuredContent.items || []

  const scroll = (direction: "left" | "right") => {
    if (!scrollRef.current) return
    const scrollAmount = 300
    if (typeof scrollRef.current.scrollBy === "function") {
      scrollRef.current.scrollBy({
        left: direction === "left" ? -scrollAmount : scrollAmount,
        behavior: "smooth",
      })
    }
  }

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "ArrowLeft") {
      e.preventDefault()
      scroll("left")
    } else if (e.key === "ArrowRight") {
      e.preventDefault()
      scroll("right")
    }
  }

  return (
    <section
      aria-label="Menu search results"
      className="flex flex-col space-y-4 rounded-2xl border border-border bg-card/60 p-4 shadow-sm backdrop-blur-xs select-text"
      onKeyDown={handleKeyDown}
    >
      {/* Header bar */}
      <header className="flex items-center justify-between gap-2 border-b border-border/60 pb-3">
        <div className="flex items-center gap-2">
          <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-primary/10 text-primary">
            <Utensils className="h-4 w-4" aria-hidden="true" />
          </div>
          <div>
            <h2 className="text-sm font-semibold text-foreground">Menu Results</h2>
            <p className="text-xs text-muted-foreground">
              {structuredContent.query_summary || `${items.length} items found`}
            </p>
          </div>
        </div>

        {/* Carousel controls */}
        {items.length > 1 && (
          <div className="flex items-center gap-1.5" aria-label="Carousel navigation">
            <button
              type="button"
              onClick={() => scroll("left")}
              aria-label="Scroll menu left"
              className="flex h-8 w-8 cursor-pointer items-center justify-center rounded-md border border-border bg-background text-foreground hover:bg-muted focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring transition-colors"
            >
              <ChevronLeft className="h-4 w-4" aria-hidden="true" />
            </button>
            <button
              type="button"
              onClick={() => scroll("right")}
              aria-label="Scroll menu right"
              className="flex h-8 w-8 cursor-pointer items-center justify-center rounded-md border border-border bg-background text-foreground hover:bg-muted focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring transition-colors"
            >
              <ChevronRight className="h-4 w-4" aria-hidden="true" />
            </button>
          </div>
        )}
      </header>

      {/* Horizontal Carousel */}
      {items.length === 0 ? (
        <div className="py-8 text-center text-sm text-muted-foreground">
          No matching dishes found for this search.
        </div>
      ) : (
        <div
          ref={scrollRef}
          role="region"
          aria-label="Menu dishes carousel. Use arrow keys to navigate."
          tabIndex={0}
          className="flex gap-3.5 overflow-x-auto pb-2 pt-1 scroll-smooth snap-x snap-mandatory no-scrollbar focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring rounded-lg"
        >
          {items.map((item) => (
            <div
              key={item.id}
              className="w-72 shrink-0 snap-start transition-transform focus-within:ring-2 focus-within:ring-ring rounded-xl"
            >
              <MenuItemCard item={item} />
            </div>
          ))}
        </div>
      )}

      {/* Persistent Allergen Notice */}
      <AllergenNotice notice={structuredContent.allergen_notice} />

      {/* Spoken transcript caption */}
      <SpokenCaption text={spokenText} />
    </section>
  )
}
