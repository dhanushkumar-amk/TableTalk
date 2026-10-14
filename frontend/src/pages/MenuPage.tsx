import { UtensilsCrossed, Search } from "lucide-react"
import { PageContainer } from "@/components/shell/PageContainer"
import { PageHeader } from "@/components/primitives/PageHeader"
import { EmptyState } from "@/components/primitives/EmptyState"
import { Button } from "@/components/ui/button"

export function MenuPage() {
  return (
    <PageContainer maxWidth="lg">
      <PageHeader
        title="Restaurant Menu"
        eyebrow="The Saffron Nebula Bistro"
        description="Freshly prepared contemporary Indian cuisine with verified allergen disclosures and spice gradings."
      />

      <EmptyState
        icon={UtensilsCrossed}
        title="Menu Browser"
        description="Fast menu exploration, dietary filter chips, and live item snapshots will load here."
        action={
          <Button variant="outline" size="default">
            <Search className="size-4 mr-2" />
            Explore Categories
          </Button>
        }
      />
    </PageContainer>
  )
}
