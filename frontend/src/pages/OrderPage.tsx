import { ShoppingBag, Plus } from "lucide-react"
import { PageContainer } from "@/components/shell/PageContainer"
import { PageHeader } from "@/components/primitives/PageHeader"
import { EmptyState } from "@/components/primitives/EmptyState"
import { Button } from "@/components/ui/button"

export function OrderPage() {
  return (
    <PageContainer maxWidth="lg">
      <PageHeader
        title="Pickup Order"
        eyebrow="Takeout & Curbside"
        description="Select dishes, specify preparation preferences, and pick up fresh from our kitchen."
      />

      <EmptyState
        icon={ShoppingBag}
        title="Your Cart is Empty"
        description="Add items from the menu to build your pickup order with live allergen conflict checking."
        action={
          <Button variant="default" size="default">
            <Plus className="size-4 mr-2" />
            Add Menu Items
          </Button>
        }
      />
    </PageContainer>
  )
}
