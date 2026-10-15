import { useState } from "react"
import { MENU_ITEMS } from "@/lib/menuData"

interface DishArtworkProps {
  categoryId?: number
  name: string
  dishId?: number
  className?: string
}

export function DishArtwork({
  categoryId: explicitCategoryId,
  name,
  dishId: explicitDishId,
  className = "",
}: DishArtworkProps) {
  const [imageError, setImageError] = useState(false)

  // Look up dish by explicit ID or name
  const matchedItem = explicitDishId
    ? MENU_ITEMS.find((m) => m.id === explicitDishId)
    : MENU_ITEMS.find((m) => m.name.toLowerCase() === name.toLowerCase())

  const categoryId = explicitCategoryId ?? matchedItem?.categoryId ?? 1
  const dishId = explicitDishId ?? matchedItem?.id

  const imageSrc = dishId ? `/images/dishes/${dishId}.jpg` : null

  // Generate deterministic subtle variations based on dish name for SVG fallback
  const hash = name.split("").reduce((acc, char) => acc + char.charCodeAt(0), 0)
  const angle = (hash % 60) - 30

  return (
    <div
      className={`relative w-full overflow-hidden bg-muted/40 flex items-center justify-center select-none ${className}`}
      aria-hidden="true"
    >
      {/* 1. Real Photography Display */}
      {imageSrc && !imageError ? (
        <div className="relative h-full w-full overflow-hidden">
          <img
            src={imageSrc}
            alt={name}
            loading="lazy"
            decoding="async"
            onError={() => setImageError(true)}
            className="h-full w-full object-cover transition-transform duration-500 ease-out group-hover:scale-105"
          />
          {/* Subtle bottom vignette to ensure floating badges and labels always pop */}
          <div className="absolute inset-0 bg-gradient-to-t from-black/40 via-transparent to-transparent pointer-events-none" />
        </div>
      ) : (
        /* 2. Graceful SVG Illustration Fallback */
        <>
          {categoryId === 1 && (
            // Starters & Chaat (Crisp, layered herbs, pomegranate burst)
            <svg viewBox="0 0 200 120" className="w-full h-full opacity-90 transition-transform duration-300 group-hover:scale-105">
              <defs>
                <linearGradient id="starterGrad" x1="0%" y1="0%" x2="100%" y2="100%">
                  <stop offset="0%" stopColor="oklch(0.56 0.165 38 / 0.25)" />
                  <stop offset="100%" stopColor="oklch(0.68 0.155 38 / 0.05)" />
                </linearGradient>
              </defs>
              <rect width="200" height="120" fill="url(#starterGrad)" />
              <circle cx="100" cy="60" r="38" className="fill-primary/15 stroke-primary/30" strokeWidth="1.5" />
              <circle cx="100" cy="60" r="24" className="fill-primary/25" />
              <path d="M75 60 Q100 35 125 60 Q100 85 75 60 Z" className="fill-foreground/10" transform={`rotate(${angle} 100 60)`} />
              <circle cx="85" cy="50" r="3.5" className="fill-primary" />
              <circle cx="112" cy="72" r="2.5" className="fill-primary/70" />
              <circle cx="118" cy="48" r="3" className="fill-primary/80" />
            </svg>
          )}

          {categoryId === 2 && (
            // Tandoor & Grills (Charcoal embers, skewers, saffron flame)
            <svg viewBox="0 0 200 120" className="w-full h-full opacity-90 transition-transform duration-300 group-hover:scale-105">
              <defs>
                <linearGradient id="grillGrad" x1="0%" y1="0%" x2="100%" y2="100%">
                  <stop offset="0%" stopColor="oklch(0.56 0.165 38 / 0.3)" />
                  <stop offset="100%" stopColor="oklch(0.45 0.12 30 / 0.1)" />
                </linearGradient>
              </defs>
              <rect width="200" height="120" fill="url(#grillGrad)" />
              <line x1="40" y1="90" x2="160" y2="30" className="stroke-foreground/30" strokeWidth="2.5" strokeDasharray="3 3" />
              <rect x="75" y="48" width="22" height="22" rx="4" className="fill-primary/40 stroke-primary/50" strokeWidth="1" transform={`rotate(20 86 59)`} />
              <rect x="105" y="38" width="22" height="22" rx="4" className="fill-primary/60 stroke-primary" strokeWidth="1" transform={`rotate(15 116 49)`} />
              <circle cx="100" cy="80" r="14" className="fill-primary/20 blur-xs" />
            </svg>
          )}

          {categoryId === 3 && (
            // Curries & Mains (Handi bowl, concentric ripples, aromatic steam)
            <svg viewBox="0 0 200 120" className="w-full h-full opacity-90 transition-transform duration-300 group-hover:scale-105">
              <defs>
                <linearGradient id="curryGrad" x1="0%" y1="0%" x2="100%" y2="100%">
                  <stop offset="0%" stopColor="oklch(0.56 0.165 38 / 0.2)" />
                  <stop offset="100%" stopColor="oklch(0.68 0.155 38 / 0.08)" />
                </linearGradient>
              </defs>
              <rect width="200" height="120" fill="url(#curryGrad)" />
              <ellipse cx="100" cy="72" rx="46" ry="24" className="fill-card stroke-border" strokeWidth="1.5" />
              <ellipse cx="100" cy="68" rx="38" ry="18" className="fill-primary/30" />
              <ellipse cx="100" cy="66" rx="26" ry="11" className="fill-primary/50" />
              <path d="M92 48 Q96 40 93 32" className="stroke-primary/40" strokeWidth="1.5" fill="none" strokeLinecap="round" />
              <path d="M102 46 Q106 38 103 30" className="stroke-primary/40" strokeWidth="1.5" fill="none" strokeLinecap="round" />
            </svg>
          )}

          {categoryId === 4 && (
            // Biryani & Artisanal Breads (Grain vessel, basmati arches)
            <svg viewBox="0 0 200 120" className="w-full h-full opacity-90 transition-transform duration-300 group-hover:scale-105">
              <defs>
                <linearGradient id="breadGrad" x1="0%" y1="0%" x2="100%" y2="100%">
                  <stop offset="0%" stopColor="oklch(0.56 0.165 38 / 0.22)" />
                  <stop offset="100%" stopColor="oklch(0.75 0.12 75 / 0.1)" />
                </linearGradient>
              </defs>
              <rect width="200" height="120" fill="url(#breadGrad)" />
              <path d="M60 85 C60 45 140 45 140 85 Z" className="fill-primary/25 stroke-primary/40" strokeWidth="1.5" />
              <ellipse cx="100" cy="85" rx="40" ry="12" className="fill-foreground/10" />
              <circle cx="100" cy="48" r="6" className="fill-primary/70" />
              <line x1="85" y1="72" x2="115" y2="72" className="stroke-primary/40" strokeWidth="1.5" strokeDasharray="2 3" />
            </svg>
          )}

          {categoryId === 5 && (
            // Desserts (Dumpling bowl, floral sweetness)
            <svg viewBox="0 0 200 120" className="w-full h-full opacity-90 transition-transform duration-300 group-hover:scale-105">
              <defs>
                <linearGradient id="dessertGrad" x1="0%" y1="0%" x2="100%" y2="100%">
                  <stop offset="0%" stopColor="oklch(0.56 0.165 38 / 0.2)" />
                  <stop offset="100%" stopColor="oklch(0.6 0.18 350 / 0.08)" />
                </linearGradient>
              </defs>
              <rect width="200" height="120" fill="url(#dessertGrad)" />
              <ellipse cx="100" cy="80" rx="36" ry="14" className="fill-card stroke-border" strokeWidth="1" />
              <circle cx="92" cy="65" r="13" className="fill-primary/50" />
              <circle cx="108" cy="65" r="13" className="fill-primary/70" />
              <circle cx="100" cy="56" r="11" className="fill-primary/30" />
              <circle cx="100" cy="54" r="2" className="fill-amber-300" />
            </svg>
          )}

          {categoryId === 6 && (
            // Beverages & Chai (Tumbler glass, steam rings, spiced cooling)
            <svg viewBox="0 0 200 120" className="w-full h-full opacity-90 transition-transform duration-300 group-hover:scale-105">
              <defs>
                <linearGradient id="bevGrad" x1="0%" y1="0%" x2="100%" y2="100%">
                  <stop offset="0%" stopColor="oklch(0.56 0.165 38 / 0.25)" />
                  <stop offset="100%" stopColor="oklch(0.65 0.14 180 / 0.08)" />
                </linearGradient>
              </defs>
              <rect width="200" height="120" fill="url(#bevGrad)" />
              <polygon points="86,35 114,35 110,88 90,88" className="fill-card stroke-border" strokeWidth="1.5" />
              <polygon points="88,48 112,48 109,85 91,85" className="fill-primary/40" />
              <line x1="84" y1="35" x2="116" y2="35" className="stroke-foreground/30" strokeWidth="2" strokeLinecap="round" />
              <circle cx="100" cy="66" r="4" className="fill-foreground/10" />
            </svg>
          )}
        </>
      )}
    </div>
  )
}
