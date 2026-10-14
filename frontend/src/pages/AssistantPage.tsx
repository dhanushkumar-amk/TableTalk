import { MessageSquare, Mic } from "lucide-react"
import { PageContainer } from "@/components/shell/PageContainer"
import { PageHeader } from "@/components/primitives/PageHeader"
import { EmptyState } from "@/components/primitives/EmptyState"
import { StatusBadge } from "@/components/primitives/StatusBadge"
import { Button } from "@/components/ui/button"

export function AssistantPage() {
  return (
    <PageContainer maxWidth="md">
      <PageHeader
        title="Voice Assistant"
        eyebrow="Interactive Session"
        description="Speak or type naturally to check table availability, reserve dining slots, or order pickup."
        badge={<StatusBadge status="neutral" label="Ready to Listen" icon={Mic} />}
      />

      <EmptyState
        icon={MessageSquare}
        title="How can I help you dine today?"
        description="Try asking 'Can we get a table for 4 tomorrow at 7:30?' or 'Show me vegan options without peanuts'."
        action={
          <Button variant="default" size="default">
            <Mic className="size-4 mr-2" />
            Start Voice Conversation
          </Button>
        }
      />
    </PageContainer>
  )
}
