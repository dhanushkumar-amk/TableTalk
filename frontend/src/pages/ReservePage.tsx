import { Calendar, Users } from "lucide-react"
import { PageContainer } from "@/components/shell/PageContainer"
import { PageHeader } from "@/components/primitives/PageHeader"
import { EmptyState } from "@/components/primitives/EmptyState"
import { Button } from "@/components/ui/button"

export function ReservePage() {
  return (
    <PageContainer maxWidth="md">
      <PageHeader
        title="Reserve a Table"
        eyebrow="Dining Reservations"
        description="Select your party size, date, preferred dining area, and meal service time."
      />

      <EmptyState
        icon={Calendar}
        title="Step-Based Booking Flow"
        description="Real-time capacity checks, slot negotiation, deposit previews, and calendar confirmation."
        action={
          <Button variant="default" size="default">
            <Users className="size-4 mr-2" />
            Check Availability
          </Button>
        }
      />
    </PageContainer>
  )
}
