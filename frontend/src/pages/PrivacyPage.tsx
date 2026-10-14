import { Shield, Lock } from "lucide-react"
import { PageContainer } from "@/components/shell/PageContainer"
import { PageHeader } from "@/components/primitives/PageHeader"
import { EmptyState } from "@/components/primitives/EmptyState"
import { Button } from "@/components/ui/button"

export function PrivacyPage() {
  return (
    <PageContainer maxWidth="md">
      <PageHeader
        title="Your Data, Your Control"
        eyebrow="Privacy & Preferences"
        description="Review stored dining preferences, rotate tokens, export your data, or permanently erase your memory records."
      />

      <EmptyState
        icon={Shield}
        title="Privacy-First Memory Controls"
        description="Memory is strictly opt-in. Stored profiles are encrypted and can be hard-deleted at any moment."
        action={
          <Button variant="outline" size="default">
            <Lock className="size-4 mr-2" />
            Manage Consent
          </Button>
        }
      />
    </PageContainer>
  )
}
