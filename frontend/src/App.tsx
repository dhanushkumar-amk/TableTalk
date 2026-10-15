import { lazy, Suspense } from "react"
import { BrowserRouter, Routes, Route } from "react-router-dom"
import { QueryClient, QueryClientProvider } from "@tanstack/react-query"
import { ThemeProvider } from "@/lib/theme"
import { AppShell } from "@/components/shell/AppShell"
import { PageSkeleton } from "@/components/shell/PageSkeleton"

// Route-level code splitting for peak performance and minimal initial bundle size
const HomePage = lazy(() => import("@/pages/HomePage").then((m) => ({ default: m.HomePage })))
const AssistantPage = lazy(() => import("@/pages/AssistantPage").then((m) => ({ default: m.AssistantPage })))
const MenuPage = lazy(() => import("@/pages/MenuPage").then((m) => ({ default: m.MenuPage })))
const ReservePage = lazy(() => import("@/pages/ReservePage").then((m) => ({ default: m.ReservePage })))
const OrderPage = lazy(() => import("@/pages/OrderPage").then((m) => ({ default: m.OrderPage })))
const ManagePage = lazy(() => import("@/pages/ManagePage").then((m) => ({ default: m.ManagePage })))
const PrivacyPage = lazy(() => import("@/pages/PrivacyPage").then((m) => ({ default: m.PrivacyPage })))
const CheckoutRoute = lazy(() => import("@/pages/CheckoutRoute").then((m) => ({ default: m.CheckoutRoute })))
const DevCardsRoute = lazy(() => import("@/pages/DevCardsRoute").then((m) => ({ default: m.DevCardsRoute })))
const NotFoundPage = lazy(() => import("@/pages/NotFoundPage").then((m) => ({ default: m.NotFoundPage })))

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 1000 * 60 * 5, // 5 minutes
      refetchOnWindowFocus: false,
      retry: 1,
    },
  },
})

export default function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <ThemeProvider>
        <BrowserRouter>
          <Suspense fallback={<PageSkeleton />}>
            <Routes>
              <Route element={<AppShell />}>
                <Route path="/" element={<HomePage />} />
                <Route path="/assistant" element={<AssistantPage />} />
                <Route path="/menu" element={<MenuPage />} />
                <Route path="/reserve" element={<ReservePage />} />
                <Route path="/order" element={<OrderPage />} />
                <Route path="/manage" element={<ManagePage />} />
                <Route path="/privacy" element={<PrivacyPage />} />
                <Route path="/checkout/:token" element={<CheckoutRoute />} />
                <Route path="/dev/cards" element={<DevCardsRoute />} />
                <Route path="*" element={<NotFoundPage />} />
              </Route>
            </Routes>
          </Suspense>
        </BrowserRouter>
      </ThemeProvider>
    </QueryClientProvider>
  )
}
