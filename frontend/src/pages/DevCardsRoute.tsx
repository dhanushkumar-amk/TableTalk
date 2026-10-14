import { lazy, Suspense } from "react"

const CardGallery = lazy(() => import("@/dev/CardGallery"))

export function DevCardsRoute() {
  return (
    <Suspense
      fallback={
        <div className="min-h-[50vh] flex items-center justify-center p-6 text-xs text-muted-foreground">
          Loading Card Gallery...
        </div>
      }
    >
      <CardGallery />
    </Suspense>
  )
}
