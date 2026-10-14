import { useState, useEffect, useRef, useMemo } from "react"
import { useSearchParams, useNavigate } from "react-router-dom"
import { useQuery } from "@tanstack/react-query"
import { motion, AnimatePresence, useReducedMotion } from "framer-motion"
import {
  Search,
  SlidersHorizontal,
  Flame,
  Clock,
  Sparkles,
  X,
  ChevronRight,
  RotateCcw,
  UtensilsCrossed,
  MessageSquare,
  ShieldAlert,
  Check,
} from "lucide-react"
import { Button } from "@/components/ui/button"
import { Money } from "@/components/primitives/Money"
import { EmptyState } from "@/components/primitives/EmptyState"
import { DishArtwork } from "@/components/menu/DishArtwork"
import {
  MENU_ITEMS,
  MENU_CATEGORIES,
  ALL_DIETARY_TAGS,
  ALL_ALLERGENS,
  SERVER_ALLERGEN_NOTICE,
  type MenuItem,
} from "@/lib/menuData"

export function MenuPage() {
  const [searchParams, setSearchParams] = useSearchParams()
  const navigate = useNavigate()
  const shouldReduceMotion = useReducedMotion()
  const searchInputRef = useRef<HTMLInputElement | null>(null)

  // URL synced state
  const query = searchParams.get("q") || ""
  const selectedCategory = searchParams.get("cat") || "all"
  const selectedDietary = useMemo(
    () => searchParams.get("diet")?.split(",").filter(Boolean) || [],
    [searchParams]
  )
  const excludedAllergens = useMemo(
    () => searchParams.get("exclude")?.split(",").filter(Boolean) || [],
    [searchParams]
  )
  const maxPrice = Number(searchParams.get("maxPrice")) || 800
  const showUnavailable = searchParams.get("unavailable") === "true"
  const sortOrder = searchParams.get("sort") || "relevance"

  // UI state
  const [showFilterDrawer, setShowFilterDrawer] = useState(false)
  const [activeItemDetail, setActiveItemDetail] = useState<MenuItem | null>(null)

  // "/" Keyboard shortcut to focus search
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (
        e.key === "/" &&
        !["INPUT", "TEXTAREA"].includes((e.target as HTMLElement).tagName)
      ) {
        e.preventDefault()
        searchInputRef.current?.focus()
      }
    }
    window.addEventListener("keydown", handleKeyDown)
    return () => window.removeEventListener("keydown", handleKeyDown)
  }, [])

  // Update query params helper
  const updateParams = (updates: Record<string, string | null>) => {
    setSearchParams(
      (prev) => {
        const next = new URLSearchParams(prev)
        Object.entries(updates).forEach(([key, val]) => {
          if (val === null || val === "" || val === "all") {
            next.delete(key)
          } else {
            next.set(key, val)
          }
        })
        return next
      },
      { replace: true }
    )
  }

  // TanStack Query for simulated fast menu fetching
  const { data: items = [], isLoading } = useQuery({
    queryKey: ["menuItems"],
    queryFn: async () => {
      // Simulate fast network fetch (40ms)
      await new Promise((r) => setTimeout(r, 40))
      return MENU_ITEMS
    },
    staleTime: 1000 * 60 * 10,
  })

  // Filtered and Sorted Menu Items
  const filteredItems = useMemo(() => {
    return items.filter((item) => {
      // 1. Category Filter
      if (selectedCategory !== "all") {
        const cat = MENU_CATEGORIES.find((c) => c.slug === selectedCategory)
        if (cat && item.categoryId !== cat.id) return false
      }

      // 2. Search Query (name or description)
      if (query.trim()) {
        const q = query.toLowerCase()
        const matchName = item.name.toLowerCase().includes(q)
        const matchDesc = item.description.toLowerCase().includes(q)
        if (!matchName && !matchDesc) return false
      }

      // 3. Dietary tags filter (must have all selected tags)
      if (selectedDietary.length > 0) {
        const hasAllTags = selectedDietary.every((tag) =>
          item.dietaryTags.includes(tag)
        )
        if (!hasAllTags) return false
      }

      // 4. Exclude allergens filter (must NOT contain any excluded allergens)
      if (excludedAllergens.length > 0) {
        const hasExcluded = excludedAllergens.some((allergen) =>
          item.allergens.includes(allergen)
        )
        if (hasExcluded) return false
      }

      // 5. Max price filter
      const priceInr = item.priceMinor / 100
      if (priceInr > maxPrice) return false

      // 6. Availability filter
      if (!showUnavailable && !item.isAvailable) return false

      return true
    }).sort((a, b) => {
      switch (sortOrder) {
        case "price_asc":
          return a.priceMinor - b.priceMinor
        case "price_desc":
          return b.priceMinor - a.priceMinor
        case "spice_asc":
          return a.spiceLevel - b.spiceLevel
        case "spice_desc":
          return b.spiceLevel - a.spiceLevel
        default:
          return 0
      }
    })
  }, [
    items,
    selectedCategory,
    query,
    selectedDietary,
    excludedAllergens,
    maxPrice,
    showUnavailable,
    sortOrder,
  ])

  // Reset all filters
  const clearFilters = () => {
    setSearchParams(new URLSearchParams(), { replace: true })
  }

  // Active filter count for badge
  const activeFiltersCount =
    (selectedDietary.length > 0 ? selectedDietary.length : 0) +
    (excludedAllergens.length > 0 ? excludedAllergens.length : 0) +
    (maxPrice < 800 ? 1 : 0) +
    (showUnavailable ? 1 : 0)

  return (
    <div className="relative min-h-[calc(100vh-3.5rem)] bg-background pb-16">
      {/* ========================================================= */}
      {/* 1. STICKY SUB-HEADER                                      */}
      {/* ========================================================= */}
      <div className="sticky top-14 z-30 border-b border-border/80 bg-background/95 backdrop-blur-md px-4 py-3">
        <div className="mx-auto max-w-7xl space-y-3">
          {/* Search Bar + Filter Trigger */}
          <div className="flex items-center gap-2.5">
            <div className="relative flex-1">
              <Search className="absolute left-3.5 top-1/2 size-4 -translate-y-1/2 text-muted-foreground pointer-events-none" />
              <input
                ref={searchInputRef}
                type="text"
                value={query}
                onChange={(e) => updateParams({ q: e.target.value })}
                placeholder="Search dishes, ingredients, curries... (Press '/' to focus)"
                className="h-10 w-full rounded-xl border border-border/80 bg-card pl-10 pr-9 text-xs sm:text-sm text-foreground placeholder:text-muted-foreground focus:border-ring focus:outline-none focus:ring-2 focus:ring-ring/20 transition-all"
              />
              {query && (
                <button
                  type="button"
                  onClick={() => updateParams({ q: null })}
                  className="absolute right-2.5 top-1/2 -translate-y-1/2 rounded-md p-1 text-muted-foreground hover:text-foreground"
                  aria-label="Clear search query"
                >
                  <X className="size-3.5" />
                </button>
              )}
            </div>

            {/* Filter Drawer Trigger Button */}
            <Button
              variant="outline"
              size="default"
              onClick={() => setShowFilterDrawer(true)}
              className="h-10 shrink-0 gap-1.5 px-3 text-xs font-medium border-border/80"
              aria-label="Open Filters"
            >
              <SlidersHorizontal className="size-3.5 text-primary" />
              <span className="hidden sm:inline">Filters</span>
              {activeFiltersCount > 0 && (
                <span className="flex size-4 items-center justify-center rounded-full bg-primary text-[10px] font-semibold text-primary-foreground">
                  {activeFiltersCount}
                </span>
              )}
            </Button>
          </div>

          {/* Horizontally Scrollable Category Pills */}
          <div
            className="flex items-center gap-1.5 overflow-x-auto pb-1 no-scrollbar text-xs"
            role="tablist"
            aria-label="Menu Categories"
          >
            {MENU_CATEGORIES.map((cat) => {
              const isSelected = selectedCategory === cat.slug
              return (
                <button
                  key={cat.slug}
                  type="button"
                  role="tab"
                  aria-selected={isSelected}
                  onClick={() => updateParams({ cat: cat.slug })}
                  className={`shrink-0 rounded-full px-3.5 py-1.5 font-medium transition-all focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring ${
                    isSelected
                      ? "bg-primary text-primary-foreground shadow-2xs"
                      : "border border-border/70 bg-card text-muted-foreground hover:border-primary/40 hover:text-foreground"
                  }`}
                >
                  {cat.name}
                </button>
              )
            })}
          </div>
        </div>
      </div>

      {/* ========================================================= */}
      {/* 2. MAIN CONTENT GRID                                      */}
      {/* ========================================================= */}
      <main className="mx-auto max-w-7xl px-4 pt-6 space-y-6">
        {/* Results Counter & Active Filter Pills */}
        <div className="flex flex-wrap items-center justify-between gap-3 text-xs text-muted-foreground">
          <div className="flex items-center gap-2">
            <span>
              Showing <strong className="text-foreground">{filteredItems.length}</strong> {filteredItems.length === 1 ? "dish" : "dishes"}
            </span>
            {activeFiltersCount > 0 && (
              <button
                type="button"
                onClick={clearFilters}
                className="text-primary hover:underline font-medium"
              >
                Clear all filters
              </button>
            )}
          </div>

          {/* Sort selector */}
          <div className="flex items-center gap-2">
            <label htmlFor="sort-select" className="text-xs">
              Sort by:
            </label>
            <select
              id="sort-select"
              value={sortOrder}
              onChange={(e) => updateParams({ sort: e.target.value })}
              className="rounded-lg border border-border/80 bg-card px-2.5 py-1 text-xs text-foreground focus:outline-none focus:ring-1 focus:ring-ring"
            >
              <option value="relevance">Relevance</option>
              <option value="price_asc">Price: Low to High</option>
              <option value="price_desc">Price: High to Low</option>
              <option value="spice_asc">Spice: Mild to High</option>
              <option value="spice_desc">Spice: High to Mild</option>
            </select>
          </div>
        </div>

        {/* Loading Skeletons */}
        {isLoading && (
          <div className="grid grid-cols-1 gap-5 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4">
            {Array.from({ length: 8 }).map((_, i) => (
              <div
                key={i}
                className="overflow-hidden rounded-2xl border border-border/80 bg-card p-4 space-y-3 animate-pulse"
              >
                <div className="h-32 w-full rounded-xl bg-muted" />
                <div className="h-4 w-2/3 rounded-sm bg-muted" />
                <div className="h-3 w-full rounded-sm bg-muted/60" />
                <div className="h-3 w-1/2 rounded-sm bg-muted/60" />
              </div>
            ))}
          </div>
        )}

        {/* Empty State when no dishes match */}
        {!isLoading && filteredItems.length === 0 && (
          <div className="py-12">
            <EmptyState
              icon={UtensilsCrossed}
              title="No matching dishes found"
              description="No menu items matched your active search or allergy exclusion criteria."
              action={
                <Button variant="outline" onClick={clearFilters}>
                  <RotateCcw className="size-4 mr-2" />
                  Reset all filters
                </Button>
              }
            />
          </div>
        )}

        {/* Menu Cards Grid */}
        {!isLoading && filteredItems.length > 0 && (
          <div className="grid grid-cols-1 gap-5 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4">
            {filteredItems.map((item) => (
              <motion.div
                key={item.id}
                layout={!shouldReduceMotion}
                initial={shouldReduceMotion ? { opacity: 1 } : { opacity: 0, scale: 0.98 }}
                animate={{ opacity: 1, scale: 1 }}
                transition={{ duration: 0.2 }}
                onClick={() => setActiveItemDetail(item)}
                className={`group relative flex flex-col justify-between overflow-hidden rounded-2xl border border-border/80 bg-card p-4 shadow-xs transition-all hover:border-border hover:shadow-sm cursor-pointer ${
                  !item.isAvailable ? "opacity-60 bg-muted/20" : ""
                }`}
              >
                <div className="space-y-3">
                  {/* Dish Illustration Header */}
                  <div className="relative overflow-hidden rounded-xl">
                    <DishArtwork
                      categoryId={item.categoryId}
                      name={item.name}
                      className="h-32 rounded-xl"
                    />

                    {/* Chef's Special Badge */}
                    {item.isChefsSpecial && (
                      <span className="absolute left-2.5 top-2.5 inline-flex items-center gap-1 rounded-full border border-primary/30 bg-background/90 px-2 py-0.5 text-[10px] font-medium text-primary backdrop-blur-xs">
                        <Sparkles className="size-2.5 text-primary" />
                        Chef's Special
                      </span>
                    )}

                    {/* Unavailable Notice */}
                    {!item.isAvailable && (
                      <span className="absolute right-2.5 top-2.5 inline-flex items-center rounded-full bg-destructive/10 px-2 py-0.5 text-[10px] font-medium text-destructive border border-destructive/20 backdrop-blur-xs">
                        Unavailable today
                      </span>
                    )}
                  </div>

                  {/* Title & Price Header */}
                  <div className="flex items-start justify-between gap-2 pt-1">
                    <h3 className="text-base font-semibold text-foreground tracking-tight group-hover:text-primary transition-colors">
                      {item.name}
                    </h3>
                    <div className="shrink-0 text-sm font-semibold text-foreground">
                      <Money amountMinor={item.priceMinor} currency="INR" />
                    </div>
                  </div>

                  {/* 2-line Description */}
                  <p className="text-xs text-muted-foreground line-clamp-2 leading-relaxed">
                    {item.description}
                  </p>

                  {/* Dietary Chips */}
                  <div className="flex flex-wrap items-center gap-1.5 pt-1">
                    {item.dietaryTags.map((tag) => (
                      <span
                        key={tag}
                        className="rounded-md border border-border/80 bg-muted/40 px-1.5 py-0.5 text-[10px] font-medium text-foreground/80"
                      >
                        {tag.replace("_", "-")}
                      </span>
                    ))}
                  </div>

                  {/* Neutral Allergen Row */}
                  <div className="rounded-lg border border-border/60 bg-muted/20 px-2.5 py-1.5 text-[11px] text-muted-foreground">
                    <span className="font-medium text-foreground/90">
                      Listed as containing:{" "}
                    </span>
                    <span>
                      {item.allergens.length > 0
                        ? item.allergens.map((a) => a.replace("_", " ")).join(", ")
                        : "none declared"}
                    </span>
                  </div>
                </div>

                {/* Footer: Spice Level + Calories + Prep time */}
                <div className="mt-4 flex items-center justify-between border-t border-border/60 pt-3 text-[11px] text-muted-foreground">
                  <div className="flex items-center gap-1" title={`Spice Level: ${item.spiceLevel} of 3`}>
                    <Flame
                      className={`size-3.5 ${
                        item.spiceLevel > 0 ? "text-primary" : "text-muted-foreground/40"
                      }`}
                    />
                    <span>
                      {item.spiceLevel === 0
                        ? "Mild"
                        : item.spiceLevel === 1
                          ? "Medium"
                          : item.spiceLevel === 2
                            ? "Hot"
                            : "Extra Hot"}
                    </span>
                  </div>

                  <div className="flex items-center gap-3">
                    <span>{item.calories} kcal</span>
                    <span className="text-border">·</span>
                    <div className="flex items-center gap-1">
                      <Clock className="size-3" />
                      <span>{item.prepTimeMinutes}m</span>
                    </div>
                  </div>
                </div>
              </motion.div>
            ))}
          </div>
        )}
      </main>

      {/* ========================================================= */}
      {/* 3. RESPONSIVE FILTER DRAWER / MODAL                       */}
      {/* ========================================================= */}
      <AnimatePresence>
        {showFilterDrawer && (
          <div
            role="dialog"
            aria-modal="true"
            aria-labelledby="filters-title"
            className="fixed inset-0 z-50 flex items-center justify-end bg-background/80 backdrop-blur-xs"
          >
            <motion.div
              initial={shouldReduceMotion ? { opacity: 1 } : { x: "100%" }}
              animate={{ x: 0 }}
              exit={shouldReduceMotion ? { opacity: 0 } : { x: "100%" }}
              transition={{ duration: 0.25, ease: "easeOut" }}
              className="relative flex h-full w-full max-w-md flex-col border-l border-border bg-card p-6 shadow-2xl overflow-y-auto"
            >
              {/* Header */}
              <div className="flex items-center justify-between pb-4 border-b border-border">
                <div className="flex items-center gap-2">
                  <SlidersHorizontal className="size-4 text-primary" />
                  <h2 id="filters-title" className="text-base font-semibold text-foreground">
                    Menu Filters
                  </h2>
                </div>
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => setShowFilterDrawer(false)}
                  className="h-8 w-8 p-0"
                  aria-label="Close filters"
                >
                  <X className="size-4" />
                </Button>
              </div>

              <div className="space-y-6 py-6 flex-1 text-sm">
                {/* Dietary Tags Multi-select */}
                <div className="space-y-2.5">
                  <h3 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
                    Dietary Requirements
                  </h3>
                  <div className="flex flex-wrap gap-2">
                    {ALL_DIETARY_TAGS.map((tag) => {
                      const isSelected = selectedDietary.includes(tag.id)
                      return (
                        <button
                          key={tag.id}
                          type="button"
                          onClick={() => {
                            const next = isSelected
                              ? selectedDietary.filter((t) => t !== tag.id)
                              : [...selectedDietary, tag.id]
                            updateParams({
                              diet: next.length > 0 ? next.join(",") : null,
                            })
                          }}
                          className={`flex min-h-[36px] items-center gap-1.5 rounded-lg px-3 py-1.5 text-xs font-medium transition-all ${
                            isSelected
                              ? "bg-primary text-primary-foreground shadow-2xs"
                              : "border border-border/80 bg-muted/30 text-muted-foreground hover:text-foreground"
                          }`}
                        >
                          {isSelected && <Check className="size-3" />}
                          <span>{tag.label}</span>
                        </button>
                      )
                    })}
                  </div>
                </div>

                {/* Exclude Allergens Multi-select */}
                <div className="space-y-2.5">
                  <div className="flex items-center justify-between">
                    <h3 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
                      Exclude Allergens
                    </h3>
                    <span className="text-[11px] text-muted-foreground">
                      Hides dishes containing
                    </span>
                  </div>
                  <div className="flex flex-wrap gap-2">
                    {ALL_ALLERGENS.map((allergen) => {
                      const isExcluded = excludedAllergens.includes(allergen.id)
                      return (
                        <button
                          key={allergen.id}
                          type="button"
                          onClick={() => {
                            const next = isExcluded
                              ? excludedAllergens.filter((a) => a !== allergen.id)
                              : [...excludedAllergens, allergen.id]
                            updateParams({
                              exclude: next.length > 0 ? next.join(",") : null,
                            })
                          }}
                          className={`flex min-h-[36px] items-center gap-1.5 rounded-lg px-3 py-1.5 text-xs font-medium transition-all ${
                            isExcluded
                              ? "border border-destructive/40 bg-destructive/10 text-destructive font-semibold"
                              : "border border-border/80 bg-muted/30 text-muted-foreground hover:text-foreground"
                          }`}
                        >
                          {isExcluded && <X className="size-3 text-destructive" />}
                          <span>{allergen.label}</span>
                        </button>
                      )
                    })}
                  </div>
                </div>

                {/* Max Price Slider */}
                <div className="space-y-2.5">
                  <div className="flex items-center justify-between text-xs">
                    <h3 className="font-semibold uppercase tracking-wider text-muted-foreground">
                      Max Price
                    </h3>
                    <span className="font-semibold text-foreground">₹{maxPrice}</span>
                  </div>
                  <input
                    type="range"
                    min="100"
                    max="800"
                    step="50"
                    value={maxPrice}
                    onChange={(e) => updateParams({ maxPrice: e.target.value })}
                    className="w-full accent-primary cursor-pointer"
                  />
                  <div className="flex justify-between text-[10px] text-muted-foreground">
                    <span>₹100</span>
                    <span>₹800</span>
                  </div>
                </div>

                {/* Show Unavailable Switch */}
                <div className="flex items-center justify-between rounded-xl border border-border/80 bg-muted/20 p-3.5">
                  <div className="space-y-0.5">
                    <label htmlFor="unavailable-toggle" className="text-xs font-medium text-foreground cursor-pointer">
                      Show Unavailable Items
                    </label>
                    <p className="text-[11px] text-muted-foreground">
                      Display dishes not currently being served today
                    </p>
                  </div>
                  <input
                    id="unavailable-toggle"
                    type="checkbox"
                    checked={showUnavailable}
                    onChange={(e) =>
                      updateParams({
                        unavailable: e.target.checked ? "true" : null,
                      })
                    }
                    className="size-4 accent-primary cursor-pointer"
                  />
                </div>
              </div>

              {/* Drawer Actions */}
              <div className="flex items-center gap-3 border-t border-border pt-4">
                <Button
                  variant="outline"
                  onClick={clearFilters}
                  className="flex-1 text-xs"
                >
                  Reset Filters
                </Button>
                <Button
                  onClick={() => setShowFilterDrawer(false)}
                  className="flex-1 text-xs"
                >
                  Apply Filters ({filteredItems.length})
                </Button>
              </div>
            </motion.div>
          </div>
        )}
      </AnimatePresence>

      {/* ========================================================= */}
      {/* 4. ITEM DETAIL SIDE SHEET / MODAL                         */}
      {/* ========================================================= */}
      <AnimatePresence>
        {activeItemDetail && (
          <div
            role="dialog"
            aria-modal="true"
            aria-labelledby="dish-detail-title"
            className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-background/80 backdrop-blur-xs"
          >
            <motion.div
              initial={shouldReduceMotion ? { opacity: 1 } : { opacity: 0, scale: 0.95 }}
              animate={{ opacity: 1, scale: 1 }}
              exit={shouldReduceMotion ? { opacity: 0 } : { opacity: 0, scale: 0.95 }}
              className="relative w-full max-w-lg rounded-2xl border border-border bg-card p-6 shadow-2xl space-y-5 max-h-[90vh] overflow-y-auto"
            >
              {/* Close Button */}
              <button
                type="button"
                onClick={() => setActiveItemDetail(null)}
                className="absolute right-4 top-4 rounded-lg p-1.5 text-muted-foreground hover:bg-muted hover:text-foreground"
                aria-label="Close detail view"
              >
                <X className="size-4" />
              </button>

              {/* Artwork */}
              <DishArtwork
                categoryId={activeItemDetail.categoryId}
                name={activeItemDetail.name}
                className="h-44 rounded-xl"
              />

              {/* Title & Category */}
              <div className="space-y-1">
                <div className="flex items-center justify-between">
                  <span className="text-xs font-medium text-primary">
                    {activeItemDetail.categoryName}
                  </span>
                  <div className="text-base font-bold text-foreground">
                    <Money amountMinor={activeItemDetail.priceMinor} currency="INR" />
                  </div>
                </div>
                <h2 id="dish-detail-title" className="text-xl font-semibold tracking-tight text-foreground">
                  {activeItemDetail.name}
                </h2>
              </div>

              {/* Full Description */}
              <p className="text-xs sm:text-sm text-muted-foreground leading-relaxed">
                {activeItemDetail.description}
              </p>

              {/* Nutrition & Specs */}
              <div className="grid grid-cols-3 gap-2.5 rounded-xl border border-border/80 bg-muted/20 p-3 text-center text-xs">
                <div>
                  <span className="block text-muted-foreground text-[10px]">Calories</span>
                  <strong className="text-foreground">{activeItemDetail.calories} kcal</strong>
                </div>
                <div>
                  <span className="block text-muted-foreground text-[10px]">Prep Time</span>
                  <strong className="text-foreground">{activeItemDetail.prepTimeMinutes} mins</strong>
                </div>
                <div>
                  <span className="block text-muted-foreground text-[10px]">Spice</span>
                  <strong className="text-foreground">
                    {activeItemDetail.spiceLevel === 0 ? "Mild" : `Level ${activeItemDetail.spiceLevel}`}
                  </strong>
                </div>
              </div>

              {/* Dietary Tags */}
              <div className="space-y-1.5">
                <span className="text-xs font-semibold text-foreground">Dietary Standards:</span>
                <div className="flex flex-wrap gap-1.5">
                  {activeItemDetail.dietaryTags.map((t) => (
                    <span
                      key={t}
                      className="rounded-md border border-border bg-muted/50 px-2 py-0.5 text-xs text-foreground"
                    >
                      {t.replace("_", "-")}
                    </span>
                  ))}
                </div>
              </div>

              {/* Allergen Disclosure & Kitchen Notice Verbatim */}
              <div className="space-y-2 rounded-xl border border-border/80 bg-muted/30 p-4 text-xs">
                <div className="flex items-center gap-2 text-foreground font-semibold">
                  <ShieldAlert className="size-4 text-primary shrink-0" />
                  <span>Allergen Disclosure</span>
                </div>

                <p className="text-muted-foreground">
                  <strong className="text-foreground">Listed as containing: </strong>
                  {activeItemDetail.allergens.length > 0
                    ? activeItemDetail.allergens.map((a) => a.replace("_", " ")).join(", ")
                    : "No common allergens declared for this recipe"}
                </p>

                {/* Verbatim Server Notice */}
                <p className="text-[11px] text-muted-foreground/90 border-t border-border/60 pt-2 italic">
                  "{SERVER_ALLERGEN_NOTICE}"
                </p>
              </div>

              {/* Action Buttons */}
              <div className="flex items-center gap-3 pt-2">
                <Button
                  variant="outline"
                  onClick={() => {
                    setActiveItemDetail(null)
                    navigate("/assistant")
                  }}
                  className="flex-1 text-xs"
                >
                  <MessageSquare className="size-3.5 mr-1.5" />
                  Ask Assistant
                </Button>
                <Button
                  onClick={() => {
                    setActiveItemDetail(null)
                    navigate("/order")
                  }}
                  className="flex-1 text-xs"
                >
                  Order Pickup
                  <ChevronRight className="size-3.5 ml-1.5" />
                </Button>
              </div>
            </motion.div>
          </div>
        )}
      </AnimatePresence>
    </div>
  )
}
