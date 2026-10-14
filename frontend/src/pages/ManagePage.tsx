import { SlidersHorizontal, Search } from "lucide-react"
import { PageContainer } from "@/components/shell/PageContainer"
import { PageHeader } from "@/components/primitives/PageHeader"
import { EmptyState } from "@/components/primitives/EmptyState"
import { Button } from "@/components/ui/button"

export function ManagePage() {
  return (
    <PageContainer maxWidth="md">
      <PageHeader
        title="Manage Activities"
        eyebrow="Guest Hub"
        description="Look up existing table reservations, pickup order status, or waitlist positions."
      />

      <EmptyState
        icon={SlidersHorizontal}
        title="Lookup Reservations & Orders"
        description="Enter your confirmation code and guest name to modify bookings or check pickup readiness."
        action={
          <Button variant="outline" size="default">
            <Search className="size-4 mr-2" />
            Lookup Booking
          </Button>
        }
      />
    </PageContainer>
  )
}
