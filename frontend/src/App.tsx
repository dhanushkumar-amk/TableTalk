import { BrowserRouter, Routes, Route } from "react-router-dom"
import { QueryClient, QueryClientProvider } from "@tanstack/react-query"
import { ThemeProvider } from "@/lib/theme"
import { AppShell } from "@/components/shell/AppShell"
import { HomePage } from "@/pages/HomePage"
import { AssistantPage } from "@/pages/AssistantPage"
import { MenuPage } from "@/pages/MenuPage"
import { ReservePage } from "@/pages/ReservePage"
import { OrderPage } from "@/pages/OrderPage"
import { ManagePage } from "@/pages/ManagePage"
import { PrivacyPage } from "@/pages/PrivacyPage"
import { CheckoutRoute } from "@/pages/CheckoutRoute"
import { DevCardsRoute } from "@/pages/DevCardsRoute"
import { NotFoundPage } from "@/pages/NotFoundPage"

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
        </BrowserRouter>
      </ThemeProvider>
    </QueryClientProvider>
  )
}
