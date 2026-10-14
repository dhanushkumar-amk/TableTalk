import { Link } from "react-router-dom"
import { MessageSquare, UtensilsCrossed, ArrowRight } from "lucide-react"
import { PageContainer } from "@/components/shell/PageContainer"
import { PageHeader } from "@/components/primitives/PageHeader"
import { Section } from "@/components/primitives/Section"
import { Button } from "@/components/ui/button"
import { StatusBadge } from "@/components/primitives/StatusBadge"

export function HomePage() {
  return (
    <PageContainer maxWidth="lg">
      <PageHeader
        title="Welcome to TableTalk"
        eyebrow="Voice-First Restaurant Assistant"
        description="A contemporary dining intelligence layer powered by the Model Context Protocol. Search menus with strict allergen safety, explore availability, and book tables naturally."
        badge={<StatusBadge status="accent" label="Phase 14 Ready" />}
        actions={
          <Button asChild size="default">
            <Link to="/assistant">
              Talk to Assistant
              <ArrowRight className="size-4 ml-1.5" />
            </Link>
          </Button>
        }
      />

      <div className="grid grid-cols-1 md:grid-cols-2 gap-6 mt-6">
        <Section
          title="Voice Assistant"
          description="Simulate real-time multi-turn voice and text conversations with The Saffron Nebula Bistro."
        >
          <p className="text-sm text-muted-foreground leading-relaxed">
            Experience natural dialogue, slot negotiation, dietary-aware ordering, and instant visual cards.
          </p>
          <div className="mt-4">
            <Button asChild variant="outline" size="sm">
              <Link to="/assistant">
                <MessageSquare className="size-3.5 mr-1.5" />
                Launch Assistant
              </Link>
            </Button>
          </div>
        </Section>

        <Section
          title="Browse Menu"
          description="Explore contemporary dishes with verified allergen disclosures and dietary filtering."
        >
          <p className="text-sm text-muted-foreground leading-relaxed">
            Every item snapshot contains real-time pricing in Indian Rupees, kitchen notes, and allergen records.
          </p>
          <div className="mt-4">
            <Button asChild variant="outline" size="sm">
              <Link to="/menu">
                <UtensilsCrossed className="size-3.5 mr-1.5" />
                View Menu
              </Link>
            </Button>
          </div>
        </Section>
      </div>
    </PageContainer>
  )
}
