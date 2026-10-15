import { Link } from "react-router-dom"
import { motion, useReducedMotion } from "framer-motion"
import { Sparkles, ArrowRight, UtensilsCrossed } from "lucide-react"

interface FeaturedDish {
  id: number
  name: string
  category: string
  price: string
  dietary: "veg" | "non-veg"
  description: string
}

const FEATURED_DISHES: FeaturedDish[] = [
  {
    id: 1,
    name: "Crisp Palak Chaat",
    category: "Small Plates",
    price: "₹450",
    dietary: "veg",
    description: "Flash-fried organic spinach, whipped yogurt, tamarind chutney, pomegranate pearls.",
  },
  {
    id: 7,
    name: "Paneer Tikka Angaare",
    category: "Tandoor & Grills",
    price: "₹595",
    dietary: "veg",
    description: "Charcoal-smoked artisanal cottage cheese, Kashmiri degi mirch, pickled onion.",
  },
  {
    id: 9,
    name: "Bhatti Ka Murgh",
    category: "Tandoor & Grills",
    price: "₹675",
    dietary: "non-veg",
    description: "Boneless chicken thigh roasted over black rock coal, whole roasted cumin.",
  },
  {
    id: 17,
    name: "Old Delhi Butter Chicken",
    category: "Curries",
    price: "₹725",
    dietary: "non-veg",
    description: "Slow-simmered vine tomato makhani, fenugreek leaf butter, charred tandoori chicken.",
  },
  {
    id: 23,
    name: "Dum Gosht Biryani",
    category: "Biryani & Rice",
    price: "₹845",
    dietary: "non-veg",
    description: "Tender goat meat, aged long-grain basmati, brown onion barista, saffron milk.",
  },
  {
    id: 28,
    name: "Saffron Baked Gulab Jamun",
    category: "Desserts",
    price: "₹395",
    dietary: "veg",
    description: "Warm golden milk dumplings baked in cardamom rabri cream, Iranian pistachios.",
  },
  {
    id: 35,
    name: "Masala Karak Chai",
    category: "Artisanal Coolers",
    price: "₹220",
    dietary: "veg",
    description: "Single-estate Assam CTC, freshly crushed ginger, cinnamon bark, green cardamom.",
  },
]

export function SignatureDishesMarquee() {
  const shouldReduceMotion = useReducedMotion()

  return (
    <section className="py-12 overflow-hidden border-t border-border/70">
      {/* Header with Kicker & View All Menu Link */}
      <div className="flex flex-col sm:flex-row sm:items-end justify-between gap-4 mb-8">
        <div>
          <div className="inline-flex items-center gap-1.5 text-xs font-mono font-semibold uppercase tracking-wider text-primary">
            <Sparkles className="size-3.5" />
            <span>CHEF'S SIGNATURE CREATIONS</span>
          </div>
          <h2 className="text-2xl sm:text-3xl font-semibold tracking-tight text-foreground mt-1">
            Real culinary craft, photographed fresh
          </h2>
        </div>
        <Link
          to="/menu"
          className="inline-flex items-center text-xs font-semibold text-primary hover:text-primary/80 transition-colors group cursor-pointer"
        >
          <UtensilsCrossed className="size-3.5 mr-1.5" />
          <span>Explore all 35 dishes</span>
          <ArrowRight className="size-3.5 ml-1 group-hover:translate-x-1 transition-transform" />
        </Link>
      </div>

      {/* Scrolling / Responsive Dish Cards Grid */}
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-5">
        {FEATURED_DISHES.slice(0, 4).map((dish, idx) => (
          <motion.div
            key={dish.id}
            initial={shouldReduceMotion ? { opacity: 1 } : { opacity: 0, y: 24 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true, margin: "-40px" }}
            transition={{ duration: 0.4, delay: shouldReduceMotion ? 0 : idx * 0.1 }}
            whileHover={shouldReduceMotion ? {} : { y: -5 }}
            className="group rounded-2xl border border-border/80 bg-card overflow-hidden shadow-xs hover:border-primary/50 hover:shadow-md transition-all flex flex-col"
          >
            {/* Real Dish Photography Canvas using CSS background */}
            <div
              className="relative h-44 w-full bg-cover bg-center overflow-hidden transition-transform duration-500 group-hover:scale-105"
              style={{
                backgroundImage: `url('/images/dishes/${dish.id}.jpg')`,
                backgroundColor: "var(--muted)",
              }}
              role="presentation"
              aria-label={dish.name}
            >
              {/* Subtle dark gradient overlay for badge readability */}
              <div className="absolute inset-0 bg-gradient-to-t from-black/60 via-transparent to-black/20" />

              {/* Price & Category Badges */}
              <div className="absolute top-3 left-3">
                <span className="px-2.5 py-1 rounded-md bg-background/90 backdrop-blur-xs text-xs font-bold font-mono text-foreground shadow-2xs">
                  {dish.price}
                </span>
              </div>

              <div className="absolute bottom-3 left-3 right-3 flex items-center justify-between text-[11px] text-white">
                <span className="font-mono uppercase tracking-wider text-[10px] bg-black/40 px-2 py-0.5 rounded backdrop-blur-xs">
                  {dish.category}
                </span>
                <span
                  className={`size-2 rounded-full ${
                    dish.dietary === "veg" ? "bg-emerald-400" : "bg-red-400"
                  }`}
                />
              </div>
            </div>

            {/* Content Details */}
            <div className="p-4 flex-1 flex flex-col justify-between space-y-2">
              <div>
                <h3 className="text-base font-semibold text-foreground tracking-tight group-hover:text-primary transition-colors">
                  {dish.name}
                </h3>
                <p className="text-xs text-muted-foreground leading-relaxed line-clamp-2 mt-1">
                  {dish.description}
                </p>
              </div>

              <div className="pt-2 border-t border-border/60 flex items-center justify-between text-[11px]">
                <span className="text-muted-foreground font-mono">100% In-House</span>
                <Link
                  to="/menu"
                  className="font-medium text-primary hover:underline underline-offset-2 flex items-center gap-0.5"
                >
                  Details <ArrowRight className="size-3" />
                </Link>
              </div>
            </div>
          </motion.div>
        ))}
      </div>
    </section>
  )
}
