import { Skeleton } from "@/components/primitives/Skeleton"

export function PageSkeleton() {
  return (
    <div
      aria-busy="true"
      aria-label="Loading page content"
      className="mx-auto max-w-4xl p-6 sm:p-8 space-y-6 animate-in fade-in duration-150"
    >
      <div className="space-y-2">
        <Skeleton className="h-8 w-48 rounded-lg" />
        <Skeleton className="h-4 w-96 max-w-full rounded-md" />
      </div>

      <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3 pt-4">
        <Skeleton className="h-36 rounded-2xl" />
        <Skeleton className="h-36 rounded-2xl" />
        <Skeleton className="h-36 rounded-2xl" />
      </div>

      <div className="pt-2">
        <Skeleton className="h-64 rounded-2xl" />
      </div>
    </div>
  )
}
