import { useState, useMemo } from "react"
import { Link } from "react-router-dom"
import { motion, AnimatePresence, useReducedMotion } from "framer-motion"
import {
  ShoppingBag,
  Plus,
  Minus,
  Trash2,
  Clock,
  ShieldAlert,
  ArrowRight,
  CheckCircle2,
  X,
} from "lucide-react"
import { toast } from "sonner"
import { Button } from "@/components/ui/button"
import { Money } from "@/components/primitives/Money"
import { CodeBlock } from "@/components/primitives/CodeBlock"
import { StatusBadge } from "@/components/primitives/StatusBadge"
import { DishArtwork } from "@/components/menu/DishArtwork"
import {
  MENU_ITEMS,
  MENU_CATEGORIES,
  ALL_ALLERGENS,
  type MenuItem,
} from "@/lib/menuData"

export interface CartLineItem {
  item: MenuItem
  quantity: number
  notes: string
}

export function OrderPage() {
  const shouldReduceMotion = useReducedMotion()

  // In-memory cart state only (no localStorage)
  const [cart, setCart] = useState<CartLineItem[]>([])
  const [selectedCategory, setSelectedCategory] = useState<number>(0)
  const [avoidedAllergens, setAvoidedAllergens] = useState<string[]>([])
  const [customerName, setCustomerName] = useState("")
  const [isMobileCartOpen, setIsMobileCartOpen] = useState(false)

  // Allergen conflict modal state
  const [allergenConflict, setAllergenConflict] = useState<{
    allergen: string
    conflictingItemNames: string[]
  } | null>(null)

  // Order Placed Confirmation State
  const [confirmedOrder, setConfirmedOrder] = useState<{
    orderCode: string
    pickupTime: string
    lines: CartLineItem[]
    totalMinor: number
    avoidedAllergens: string[]
    customerName: string
  } | null>(null)

  // Cancel order confirmation dialog
  const [showCancelDialog, setShowCancelDialog] = useState(false)

  // Filtered menu items for pickup
  const displayedItems = useMemo(() => {
    return MENU_ITEMS.filter((item) => {
      if (!item.isAvailable) return false // Pickup only includes available items
      if (selectedCategory === 0) return true
      return item.categoryId === selectedCategory
    })
  }, [selectedCategory])

  // Cart operations
  const addToCart = (item: MenuItem) => {
    // Check if item contains an allergen currently marked to avoid
    const conflict = avoidedAllergens.find((a) => item.allergens.includes(a))
    if (conflict) {
      const allergenLabel =
        ALL_ALLERGENS.find((a) => a.id === conflict)?.label || conflict
      setAllergenConflict({
        allergen: allergenLabel,
        conflictingItemNames: [item.name],
      })
      return
    }

    setCart((prev) => {
      const existing = prev.find((line) => line.item.id === item.id)
      if (existing) {
        return prev.map((line) =>
          line.item.id === item.id
            ? { ...line, quantity: line.quantity + 1 }
            : line
        )
      }
      return [...prev, { item, quantity: 1, notes: "" }]
    })

    toast.success(`Added ${item.name} to cart`)
  }

  const updateQuantity = (itemId: number, delta: number) => {
    setCart((prev) =>
      prev
        .map((line) => {
          if (line.item.id === itemId) {
            const newQty = line.quantity + delta
            return newQty > 0 ? { ...line, quantity: newQty } : null
          }
          return line
        })
        .filter((line): line is CartLineItem => line !== null)
    )
  }

  const removeLine = (itemId: number) => {
    setCart((prev) => prev.filter((line) => line.item.id !== itemId))
    toast.info("Item removed from cart")
  }

  const updateLineNotes = (itemId: number, notes: string) => {
    setCart((prev) =>
      prev.map((line) =>
        line.item.id === itemId ? { ...line, notes } : line
      )
    )
  }

  // Handle allergen toggle in cart: if toggling ON, check for existing conflicts!
  const toggleAvoidedAllergen = (allergenId: string) => {
    const isCurrentlyAvoided = avoidedAllergens.includes(allergenId)
    if (isCurrentlyAvoided) {
      setAvoidedAllergens((prev) => prev.filter((a) => a !== allergenId))
    } else {
      // Check existing cart items for conflict
      const conflicting = cart.filter((line) =>
        line.item.allergens.includes(allergenId)
      )
      if (conflicting.length > 0) {
        const allergenLabel =
          ALL_ALLERGENS.find((a) => a.id === allergenId)?.label || allergenId
        setAllergenConflict({
          allergen: allergenLabel,
          conflictingItemNames: conflicting.map((c) => c.item.name),
        })
      }
      setAvoidedAllergens((prev) => [...prev, allergenId])
    }
  }

  // Calculations
  const subtotalMinor = useMemo(() => {
    return cart.reduce((acc, line) => acc + line.item.priceMinor * line.quantity, 0)
  }, [cart])

  const taxMinor = Math.round(subtotalMinor * 0.05) // 5% GST
  const totalMinor = subtotalMinor + taxMinor
  const totalItemsCount = cart.reduce((acc, line) => acc + line.quantity, 0)

  // Max prep time calculation for pickup estimate
  const maxPrepTime = useMemo(() => {
    if (cart.length === 0) return 20
    return Math.max(...cart.map((line) => line.item.prepTimeMinutes), 15)
  }, [cart])

  const pickupTimeEstimate = useMemo(() => {
    const d = new Date(Date.now() + maxPrepTime * 60 * 1000)
    return d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })
  }, [maxPrepTime])

  // Place Order
  const handlePlaceOrder = () => {
    if (!customerName.trim()) {
      toast.error("Please enter your name for pickup order identification")
      return
    }

    if (cart.length === 0) {
      toast.error("Your cart is empty")
      return
    }

    const code = `TT-ORD-${Math.floor(1000 + Math.random() * 9000)}`
    setConfirmedOrder({
      orderCode: code,
      pickupTime: pickupTimeEstimate,
      lines: [...cart],
      totalMinor,
      avoidedAllergens: [...avoidedAllergens],
      customerName: customerName.trim(),
    })
    setCart([])
    setIsMobileCartOpen(false)
    toast.success("Order placed successfully! Pay at pickup.")
  }

  // Cancel Confirmed Order
  const handleCancelOrder = () => {
    setShowCancelDialog(false)
    setConfirmedOrder(null)
    toast.info("Pickup order cancelled")
  }

  return (
    <div className="relative min-h-[calc(100vh-3.5rem)] bg-background pb-24">
      {/* ========================================================= */}
      {/* 1. TOP HEADER / TITLE                                     */}
      {/* ========================================================= */}
      <div className="border-b border-border/80 bg-card/40 backdrop-blur-xs px-4 py-4">
        <div className="mx-auto max-w-7xl flex flex-col sm:flex-row items-start sm:items-center justify-between gap-3">
          <div>
            <h1 className="text-xl sm:text-2xl font-semibold tracking-tight text-foreground">
              Pickup Order
            </h1>
            <p className="text-xs sm:text-sm text-muted-foreground pt-0.5">
              Select dishes, verify ingredients, and pick up fresh with pay-at-pickup.
            </p>
          </div>

          <div className="flex items-center gap-2">
            <StatusBadge status="neutral" label="Pay at Pickup" />
            <div className="flex items-center gap-1.5 rounded-full border border-border/80 bg-muted/30 px-3 py-1 text-xs text-muted-foreground">
              <Clock className="size-3 text-primary" />
              <span>Est. Prep: {maxPrepTime} mins</span>
            </div>
          </div>
        </div>
      </div>

      {/* ========================================================= */}
      {/* 2. CONFIRMED ORDER SUCCESS VIEW                           */}
      {/* ========================================================= */}
      {confirmedOrder ? (
        <div className="mx-auto max-w-2xl px-4 pt-8">
          <motion.div
            initial={shouldReduceMotion ? { opacity: 1 } : { opacity: 0, y: 12 }}
            animate={{ opacity: 1, y: 0 }}
            className="rounded-2xl border border-border/80 bg-card p-6 sm:p-8 shadow-xs space-y-6"
          >
            <div className="flex items-center gap-3 text-emerald-600 dark:text-emerald-400">
              <div className="flex size-12 items-center justify-center rounded-2xl bg-emerald-500/10">
                <CheckCircle2 className="size-6 text-emerald-600 dark:text-emerald-400" />
              </div>
              <div>
                <h2 className="text-xl sm:text-2xl font-semibold tracking-tight text-foreground">
                  Pickup Order Confirmed!
                </h2>
                <p className="text-xs text-muted-foreground">
                  Our kitchen has received your ticket. Pay when you collect.
                </p>
              </div>
            </div>

            {/* Order Code */}
            <div className="space-y-1.5">
              <span className="text-xs font-medium text-muted-foreground">
                Your Order Pickup Code:
              </span>
              <CodeBlock code={confirmedOrder.orderCode} label="Order Code" />
            </div>

            {/* Summary Details */}
            <div className="rounded-xl border border-border/80 bg-muted/20 p-4 space-y-4 text-xs">
              <div className="flex items-center justify-between pb-2 border-b border-border/60">
                <span className="text-muted-foreground">Ready for Pickup</span>
                <strong className="text-foreground font-mono text-sm">{confirmedOrder.pickupTime}</strong>
              </div>

              <div className="space-y-2">
                <span className="font-semibold text-foreground">Items Ordered:</span>
                {confirmedOrder.lines.map((line, idx) => (
                  <div key={idx} className="flex items-start justify-between text-xs">
                    <div>
                      <span className="font-medium text-foreground">
                        {line.quantity}x {line.item.name}
                      </span>
                      {line.notes && (
                        <p className="text-[11px] text-muted-foreground italic pl-3">
                          "{line.notes}"
                        </p>
                      )}
                    </div>
                    <Money amountMinor={line.item.priceMinor * line.quantity} currency="INR" />
                  </div>
                ))}
              </div>

              <div className="border-t border-border/60 pt-3 flex items-center justify-between text-sm font-semibold">
                <span>Total Due at Pickup:</span>
                <Money amountMinor={confirmedOrder.totalMinor} currency="INR" />
              </div>

              {confirmedOrder.avoidedAllergens.length > 0 && (
                <div className="rounded-lg bg-card border border-border/70 p-2.5 text-[11px] text-muted-foreground">
                  <span className="font-semibold text-foreground">Stated Allergens Avoided: </span>
                  {confirmedOrder.avoidedAllergens.join(", ")}
                </div>
              )}
            </div>

            {/* Actions: Cancel Order or Manage */}
            <div className="flex flex-col sm:flex-row items-center gap-3 pt-2">
              <Button
                variant="outline"
                onClick={() => setShowCancelDialog(true)}
                className="h-11 w-full sm:flex-1 text-xs text-destructive hover:bg-destructive/10 hover:text-destructive"
              >
                Cancel Order
              </Button>
              <Button
                asChild
                className="h-11 w-full sm:flex-1 text-xs font-medium shadow-xs"
              >
                <Link to={`/manage?order=${confirmedOrder.orderCode}`}>
                  Manage in Hub
                  <ArrowRight className="size-3.5 ml-1.5" />
                </Link>
              </Button>
            </div>
          </motion.div>
        </div>
      ) : (
        /* ========================================================= */
        /* 3. ORDERING EXPERIENCE (MAIN MENU + STICKY CART)          */
        /* ========================================================= */
        <div className="mx-auto max-w-7xl px-4 pt-6">
          <div className="grid grid-cols-1 lg:grid-cols-12 gap-8 items-start">
            {/* Left/Main Column: Menu Browser (7 cols on desktop) */}
            <div className="lg:col-span-7 xl:col-span-8 space-y-5">
              {/* Category Filter Pills */}
              <div
                className="flex items-center gap-1.5 overflow-x-auto pb-1 no-scrollbar text-xs"
                role="tablist"
                aria-label="Order Categories"
              >
                {MENU_CATEGORIES.map((cat) => {
                  const isSelected = selectedCategory === cat.id
                  return (
                    <button
                      key={cat.id}
                      type="button"
                      role="tab"
                      aria-selected={isSelected}
                      onClick={() => setSelectedCategory(cat.id)}
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

              {/* Items List */}
              <div className="space-y-3">
                {displayedItems.map((item) => {
                  const cartItem = cart.find((l) => l.item.id === item.id)
                  const qtyInCart = cartItem?.quantity || 0

                  return (
                    <div
                      key={item.id}
                      className="group flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4 rounded-xl border border-border/80 bg-card p-4 shadow-2xs transition-all hover:border-border"
                    >
                      <div className="flex items-start gap-3.5">
                        <DishArtwork
                          categoryId={item.categoryId}
                          name={item.name}
                          className="size-16 sm:size-20 rounded-lg shrink-0"
                        />
                        <div className="space-y-1">
                          <div className="flex items-center gap-2">
                            <h3 className="text-sm sm:text-base font-semibold text-foreground tracking-tight">
                              {item.name}
                            </h3>
                            {item.isChefsSpecial && (
                              <span className="rounded-full bg-primary/10 px-2 py-0.2 text-[10px] font-medium text-primary">
                                Chef's Pick
                              </span>
                            )}
                          </div>
                          <p className="text-xs text-muted-foreground line-clamp-2 leading-relaxed">
                            {item.description}
                          </p>
                          <div className="flex flex-wrap items-center gap-1 pt-1">
                            {item.dietaryTags.slice(0, 3).map((tag) => (
                              <span
                                key={tag}
                                className="rounded-md border border-border/80 bg-muted/40 px-1.5 py-0.5 text-[9px] font-medium text-foreground/80"
                              >
                                {tag.replace("_", "-")}
                              </span>
                            ))}
                            <span className="text-[10px] text-muted-foreground pl-1">
                              Listed allergens: {item.allergens.length > 0 ? item.allergens.join(", ") : "none"}
                            </span>
                          </div>
                        </div>
                      </div>

                      <div className="flex sm:flex-col items-center sm:items-end justify-between w-full sm:w-auto gap-3 shrink-0 pt-2 sm:pt-0">
                        <div className="text-sm font-semibold text-foreground">
                          <Money amountMinor={item.priceMinor} currency="INR" />
                        </div>

                        {qtyInCart === 0 ? (
                          <Button
                            size="sm"
                            onClick={() => addToCart(item)}
                            className="h-8 px-3 text-xs font-medium shadow-2xs"
                          >
                            <Plus className="size-3.5 mr-1" />
                            Add
                          </Button>
                        ) : (
                          <div className="flex items-center rounded-lg border border-border/80 bg-muted/40 p-0.5">
                            <button
                              type="button"
                              onClick={() => updateQuantity(item.id, -1)}
                              className="flex size-7 items-center justify-center rounded-md hover:bg-card text-foreground"
                              aria-label={`Decrease quantity of ${item.name}`}
                            >
                              <Minus className="size-3" />
                            </button>
                            <span className="w-7 text-center text-xs font-semibold text-foreground">
                              {qtyInCart}
                            </span>
                            <button
                              type="button"
                              onClick={() => updateQuantity(item.id, 1)}
                              className="flex size-7 items-center justify-center rounded-md hover:bg-card text-foreground"
                              aria-label={`Increase quantity of ${item.name}`}
                            >
                              <Plus className="size-3" />
                            </button>
                          </div>
                        )}
                      </div>
                    </div>
                  )
                })}
              </div>
            </div>

            {/* Right Column: Sticky Cart Sidebar (5 cols on desktop) */}
            <aside className="hidden lg:block lg:col-span-5 xl:col-span-4 space-y-5">
              <div className="sticky top-20 rounded-2xl border border-border/80 bg-card p-5 shadow-xs space-y-5">
                <div className="flex items-center justify-between pb-3 border-b border-border/70">
                  <div className="flex items-center gap-2">
                    <ShoppingBag className="size-4 text-primary" />
                    <h2 className="text-sm font-semibold text-foreground">Your Pickup Cart</h2>
                  </div>
                  {cart.length > 0 && (
                    <span className="text-xs text-muted-foreground">{totalItemsCount} items</span>
                  )}
                </div>

                {cart.length === 0 ? (
                  <div className="py-8 text-center space-y-2">
                    <p className="text-xs text-muted-foreground">
                      Your pickup cart is empty. Add dishes from the menu to get started.
                    </p>
                  </div>
                ) : (
                  <div className="space-y-4">
                    {/* Cart Line Items */}
                    <div className="space-y-3 max-h-60 overflow-y-auto pr-1">
                      {cart.map((line) => (
                        <div key={line.item.id} className="rounded-xl border border-border/60 bg-muted/20 p-3 space-y-2 text-xs">
                          <div className="flex items-center justify-between">
                            <span className="font-semibold text-foreground">{line.item.name}</span>
                            <Money amountMinor={line.item.priceMinor * line.quantity} currency="INR" />
                          </div>

                          <div className="flex items-center justify-between">
                            <div className="flex items-center rounded-lg border border-border/80 bg-card p-0.5">
                              <button
                                type="button"
                                onClick={() => updateQuantity(line.item.id, -1)}
                                className="flex size-6 items-center justify-center rounded-md hover:bg-muted text-foreground"
                                aria-label="Decrease"
                              >
                                <Minus className="size-2.5" />
                              </button>
                              <span className="w-6 text-center text-xs font-semibold">{line.quantity}</span>
                              <button
                                type="button"
                                onClick={() => updateQuantity(line.item.id, 1)}
                                className="flex size-6 items-center justify-center rounded-md hover:bg-muted text-foreground"
                                aria-label="Increase"
                              >
                                <Plus className="size-2.5" />
                              </button>
                            </div>

                            <button
                              type="button"
                              onClick={() => removeLine(line.item.id)}
                              className="text-muted-foreground hover:text-destructive p-1"
                              aria-label={`Remove ${line.item.name}`}
                            >
                              <Trash2 className="size-3.5" />
                            </button>
                          </div>

                          <input
                            type="text"
                            value={line.notes}
                            onChange={(e) => updateLineNotes(line.item.id, e.target.value)}
                            placeholder="Add item note (e.g. extra chutney)..."
                            className="h-7 w-full rounded-md border border-border/70 bg-card px-2 text-[11px] text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-ring"
                          />
                        </div>
                      ))}
                    </div>

                    {/* Allergen Avoidance Multi-select */}
                    <div className="space-y-2 border-t border-border/60 pt-3">
                      <div className="flex items-center justify-between text-xs">
                        <span className="font-semibold text-foreground">Anything to avoid?</span>
                        <span className="text-[10px] text-muted-foreground">Kitchen alert</span>
                      </div>
                      <div className="flex flex-wrap gap-1.5">
                        {ALL_ALLERGENS.slice(0, 6).map((allergen) => {
                          const isAvoided = avoidedAllergens.includes(allergen.id)
                          return (
                            <button
                              key={allergen.id}
                              type="button"
                              onClick={() => toggleAvoidedAllergen(allergen.id)}
                              className={`rounded-lg px-2 py-1 text-[10px] font-medium transition-all ${
                                isAvoided
                                  ? "border border-destructive/40 bg-destructive/10 text-destructive font-semibold"
                                  : "border border-border/80 bg-muted/30 text-muted-foreground hover:text-foreground"
                              }`}
                            >
                              {isAvoided ? `Avoid: ${allergen.label}` : allergen.label}
                            </button>
                          )
                        })}
                      </div>
                    </div>

                    {/* Customer Name */}
                    <div className="space-y-1.5 border-t border-border/60 pt-3">
                      <label htmlFor="customerNameDesktop" className="text-xs font-semibold text-foreground">
                        Your Name (for pickup callout) <span className="text-destructive">*</span>
                      </label>
                      <input
                        id="customerNameDesktop"
                        type="text"
                        value={customerName}
                        onChange={(e) => setCustomerName(e.target.value)}
                        placeholder="e.g. Vikram Malhotra"
                        className="h-9 w-full rounded-xl border border-border/80 bg-muted/20 px-3 text-xs text-foreground placeholder:text-muted-foreground focus:border-ring focus:outline-none focus:ring-1 focus:ring-ring"
                      />
                    </div>

                    {/* Pricing Breakdown */}
                    <div className="space-y-2 border-t border-border/60 pt-3 text-xs">
                      <div className="flex items-center justify-between text-muted-foreground">
                        <span>Subtotal</span>
                        <Money amountMinor={subtotalMinor} currency="INR" />
                      </div>
                      <div className="flex items-center justify-between text-muted-foreground">
                        <span>GST (5%)</span>
                        <Money amountMinor={taxMinor} currency="INR" />
                      </div>
                      <div className="flex items-center justify-between text-sm font-bold text-foreground border-t border-border/60 pt-2">
                        <span>Total Due</span>
                        <Money amountMinor={totalMinor} currency="INR" />
                      </div>
                    </div>

                    {/* Submit Button */}
                    <Button
                      onClick={handlePlaceOrder}
                      className="h-11 w-full text-xs font-medium shadow-xs"
                    >
                      Place Pickup Order ({totalItemsCount})
                      <ArrowRight className="size-3.5 ml-1.5" />
                    </Button>
                  </div>
                )}
              </div>
            </aside>
          </div>
        </div>
      )}

      {/* ========================================================= */}
      {/* 4. MOBILE STICKY BOTTOM BAR                               */}
      {/* ========================================================= */}
      {cart.length > 0 && !confirmedOrder && (
        <div className="lg:hidden fixed bottom-14 left-0 right-0 z-40 p-3 bg-background/95 backdrop-blur-md border-t border-border shadow-lg">
          <Button
            onClick={() => setIsMobileCartOpen(true)}
            className="h-11 w-full flex items-center justify-between px-4 text-xs font-semibold shadow-md"
          >
            <div className="flex items-center gap-2">
              <ShoppingBag className="size-4" />
              <span>View Cart ({totalItemsCount})</span>
            </div>
            <Money amountMinor={totalMinor} currency="INR" />
          </Button>
        </div>
      )}

      {/* ========================================================= */}
      {/* 5. MOBILE CART DRAWER                                     */}
      {/* ========================================================= */}
      <AnimatePresence>
        {isMobileCartOpen && (
          <div
            role="dialog"
            aria-modal="true"
            aria-labelledby="mobile-cart-title"
            className="lg:hidden fixed inset-0 z-50 flex flex-col justify-end bg-background/80 backdrop-blur-xs"
          >
            <motion.div
              initial={shouldReduceMotion ? { opacity: 1 } : { y: "100%" }}
              animate={{ y: 0 }}
              exit={shouldReduceMotion ? { opacity: 0 } : { y: "100%" }}
              transition={{ duration: 0.25, ease: "easeOut" }}
              className="relative max-h-[85vh] rounded-t-2xl border-t border-border bg-card p-5 shadow-2xl flex flex-col overflow-y-auto space-y-4 pb-safe"
            >
              <div className="flex items-center justify-between pb-2 border-b border-border">
                <h2 id="mobile-cart-title" className="text-base font-semibold text-foreground">
                  Your Cart ({totalItemsCount})
                </h2>
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => setIsMobileCartOpen(false)}
                  className="h-8 w-8 p-0"
                >
                  <X className="size-4" />
                </Button>
              </div>

              {/* Cart Items list */}
              <div className="space-y-3 flex-1 overflow-y-auto">
                {cart.map((line) => (
                  <div key={line.item.id} className="rounded-xl border border-border/80 bg-muted/20 p-3 space-y-2 text-xs">
                    <div className="flex items-center justify-between font-semibold">
                      <span>{line.item.name}</span>
                      <Money amountMinor={line.item.priceMinor * line.quantity} currency="INR" />
                    </div>
                    <div className="flex items-center justify-between">
                      <div className="flex items-center rounded-lg border border-border bg-card p-0.5">
                        <button
                          type="button"
                          onClick={() => updateQuantity(line.item.id, -1)}
                          className="flex size-7 items-center justify-center"
                        >
                          <Minus className="size-3" />
                        </button>
                        <span className="w-7 text-center font-semibold">{line.quantity}</span>
                        <button
                          type="button"
                          onClick={() => updateQuantity(line.item.id, 1)}
                          className="flex size-7 items-center justify-center"
                        >
                          <Plus className="size-3" />
                        </button>
                      </div>
                      <button
                        type="button"
                        onClick={() => removeLine(line.item.id)}
                        className="text-destructive p-1"
                      >
                        <Trash2 className="size-4" />
                      </button>
                    </div>
                  </div>
                ))}
              </div>

              {/* Customer Name */}
              <div className="space-y-1">
                <label htmlFor="customerNameMobile" className="text-xs font-semibold text-foreground">
                  Your Name for Pickup <span className="text-destructive">*</span>
                </label>
                <input
                  id="customerNameMobile"
                  type="text"
                  value={customerName}
                  onChange={(e) => setCustomerName(e.target.value)}
                  placeholder="e.g. Vikram Malhotra"
                  className="h-10 w-full rounded-xl border border-border bg-muted/20 px-3 text-xs"
                />
              </div>

              {/* Total & Submit */}
              <div className="border-t border-border pt-3 space-y-3">
                <div className="flex items-center justify-between font-bold text-sm">
                  <span>Total Due at Pickup:</span>
                  <Money amountMinor={totalMinor} currency="INR" />
                </div>
                <Button
                  onClick={handlePlaceOrder}
                  className="h-11 w-full text-xs font-medium"
                >
                  Place Pickup Order
                </Button>
              </div>
            </motion.div>
          </div>
        )}
      </AnimatePresence>

      {/* ========================================================= */}
      {/* 6. ALLERGEN CONFLICT DIALOG (Strictly calm, no 'safe')     */}
      {/* ========================================================= */}
      <AnimatePresence>
        {allergenConflict && (
          <div
            role="dialog"
            aria-modal="true"
            aria-labelledby="conflict-dialog-title"
            className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-background/80 backdrop-blur-xs"
          >
            <motion.div
              initial={shouldReduceMotion ? { opacity: 1 } : { opacity: 0, scale: 0.95 }}
              animate={{ opacity: 1, scale: 1 }}
              exit={shouldReduceMotion ? { opacity: 0 } : { opacity: 0, scale: 0.95 }}
              className="w-full max-w-md rounded-2xl border border-border bg-card p-6 shadow-2xl space-y-4"
            >
              <div className="flex items-center gap-3 text-amber-600 dark:text-amber-400">
                <div className="flex size-10 items-center justify-center rounded-xl bg-amber-500/10">
                  <ShieldAlert className="size-5" />
                </div>
                <h3 id="conflict-dialog-title" className="text-base font-semibold text-foreground">
                  Allergen Notice: {allergenConflict.allergen}
                </h3>
              </div>

              <div className="text-xs sm:text-sm text-muted-foreground leading-relaxed space-y-2">
                <p>
                  You indicated a preference to avoid <strong>{allergenConflict.allergen}</strong>. The following item in your order is listed as containing this ingredient:
                </p>
                <ul className="list-disc pl-5 font-medium text-foreground">
                  {allergenConflict.conflictingItemNames.map((name, i) => (
                    <li key={i}>{name}</li>
                  ))}
                </ul>
                <p className="text-[11px] italic pt-1 text-muted-foreground">
                  Note: The kitchen handles common allergens; cross-contact risk may occur.
                </p>
              </div>

              <div className="flex items-center justify-end gap-2.5 pt-2">
                <Button
                  variant="outline"
                  onClick={() => {
                    // Remove conflicting items from cart
                    setCart((prev) =>
                      prev.filter(
                        (line) => !allergenConflict.conflictingItemNames.includes(line.item.name)
                      )
                    )
                    setAllergenConflict(null)
                    toast.info("Conflicting items removed from cart")
                  }}
                  className="h-9 px-4 text-xs font-medium"
                >
                  Remove Conflicting Item
                </Button>
                <Button
                  onClick={() => setAllergenConflict(null)}
                  className="h-9 px-4 text-xs font-medium"
                >
                  Keep in Order
                </Button>
              </div>
            </motion.div>
          </div>
        )}
      </AnimatePresence>

      {/* ========================================================= */}
      {/* 7. CANCEL ORDER CONFIRMATION DIALOG                       */}
      {/* ========================================================= */}
      <AnimatePresence>
        {showCancelDialog && (
          <div
            role="dialog"
            aria-modal="true"
            aria-labelledby="cancel-order-title"
            className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-background/80 backdrop-blur-xs"
          >
            <motion.div
              initial={shouldReduceMotion ? { opacity: 1 } : { opacity: 0, scale: 0.95 }}
              animate={{ opacity: 1, scale: 1 }}
              exit={shouldReduceMotion ? { opacity: 0 } : { opacity: 0, scale: 0.95 }}
              className="w-full max-w-md rounded-2xl border border-border bg-card p-6 shadow-2xl space-y-4"
            >
              <div className="flex items-center gap-3 text-destructive">
                <div className="flex size-10 items-center justify-center rounded-xl bg-destructive/10">
                  <ShieldAlert className="size-5" />
                </div>
                <h3 id="cancel-order-title" className="text-base font-semibold text-foreground">
                  Cancel Pickup Order?
                </h3>
              </div>

              <p className="text-xs sm:text-sm text-muted-foreground leading-relaxed">
                Are you sure you want to cancel order <strong>{confirmedOrder?.orderCode}</strong>? The kitchen will stop preparing your dishes.
              </p>

              <div className="flex items-center justify-end gap-2.5 pt-2">
                <Button
                  variant="outline"
                  onClick={() => setShowCancelDialog(false)}
                  className="h-9 px-4 text-xs"
                >
                  Keep Order
                </Button>
                <Button
                  variant="destructive"
                  onClick={handleCancelOrder}
                  className="h-9 px-4 text-xs shadow-xs"
                >
                  Confirm Cancellation
                </Button>
              </div>
            </motion.div>
          </div>
        )}
      </AnimatePresence>
    </div>
  )
}
