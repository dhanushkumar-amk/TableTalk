import { useEffect } from "react"
import { useLocation } from "react-router-dom"

const ROUTE_META: Record<string, { title: string; description: string }> = {
  "/": {
    title: "TableTalk — Voice-First Restaurant Assistant",
    description: "Book a table by just asking. Live table capacity, allergen-aware menu intelligence, and pickup orders.",
  },
  "/assistant": {
    title: "Voice Assistant — TableTalk",
    description: "Talk to TableTalk with natural speech or text. Search the menu, check capacity, and book a table.",
  },
  "/menu": {
    title: "Dining Menu — TableTalk",
    description: "Browse dishes, filter allergens and dietary tags, check preparation times, and view chef specialties.",
  },
  "/reserve": {
    title: "Reserve a Table — TableTalk",
    description: "Step-based table reservations with live availability, alternative recommendations, and deposit holds.",
  },
  "/order": {
    title: "Pickup Ordering — TableTalk",
    description: "Order fresh food for pickup. In-memory cart with calm allergen checking and dynamic kitchen prep estimates.",
  },
  "/manage": {
    title: "Manage Reservations & Orders — TableTalk",
    description: "Look up bookings, track pickup status timelines, or manage your waitlist position.",
  },
  "/privacy": {
    title: "Privacy & Data Sovereignty — TableTalk",
    description: "Complete control of your dining data. Opt-in memory, zero ad tracking, token rotation, and instant data purging.",
  },
  "/dev/cards": {
    title: "Card Gallery — TableTalk Dev",
    description: "Component gallery for all 33 real server output card fixtures.",
  },
}

export function usePageMeta() {
  const location = useLocation()

  useEffect(() => {
    const pathname = location.pathname

    // Exact match or prefix match for dynamic routes
    let meta = ROUTE_META[pathname]

    if (!meta) {
      if (pathname.startsWith("/checkout")) {
        meta = {
          title: "Secure Simulated Deposit — TableTalk",
          description: "Simulated table deposit checkout. No real card numbers or money required.",
        }
      } else {
        meta = {
          title: "Page Not Found — TableTalk",
          description: "The page you are looking for does not exist.",
        }
      }
    }

    document.title = meta.title

    // Update meta description
    let metaDescriptionEl = document.querySelector('meta[name="description"]')
    if (!metaDescriptionEl) {
      metaDescriptionEl = document.createElement("meta")
      metaDescriptionEl.setAttribute("name", "description")
      document.head.appendChild(metaDescriptionEl)
    }
    metaDescriptionEl.setAttribute("content", meta.description)
  }, [location.pathname])
}
