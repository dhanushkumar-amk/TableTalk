import { AlertTriangle, Info } from "lucide-react"

interface AllergenNoticeProps {
  /**
   * The verbatim allergen notice string provided by the server.
   */
  notice?: string | null

  /**
   * Specific allergens known to be present in the item or meal.
   * Rendered strictly under the phrasing "Listed as containing: ...".
   */
  containingAllergens?: string[] | null

  /**
   * Specific allergens confirmed absent in the server item record.
   * Rendered strictly under the phrasing "Listed as not containing: ...".
   */
  notContainingAllergens?: string[] | null

  /**
   * Whether the allergen status of the item or order is unverified.
   */
  isUnverified?: boolean

  className?: string
}

/**
 * AllergenNotice renders safety-critical allergen information.
 *
 * SAFETY INVARIANTS:
 * - Uses the server's allergen_notice text verbatim.
 * - Shows "Listed as containing: ..." and "Listed as not containing: ..." ONLY.
 * - NEVER uses words like "safe", "allergy-free", or "guaranteed".
 * - NEVER displays green checkmarks or green indicators that imply safety.
 * - Uses neutral amber or slate iconography and chips.
 * - Explicitly labels unverified data as "Allergen information unverified".
 */
export function AllergenNotice({
  notice,
  containingAllergens,
  notContainingAllergens,
  isUnverified = false,
  className = "",
}: AllergenNoticeProps) {
  const hasContaining = containingAllergens && containingAllergens.length > 0
  const hasNotContaining = notContainingAllergens && notContainingAllergens.length > 0

  return (
    <div
      role="region"
      aria-label="Allergen advisory"
      className={`rounded-lg border border-amber-500/30 bg-amber-500/10 p-3 text-xs text-foreground/90 transition-colors ${className}`}
    >
      {/* Header */}
      <div className="flex items-center gap-1.5 font-medium text-amber-700 dark:text-amber-400">
        <AlertTriangle className="h-4 w-4 shrink-0 text-amber-600 dark:text-amber-400" aria-hidden="true" />
        <span className="font-semibold tracking-tight">Kitchen Allergen Advisory</span>
      </div>

      {/* Unverified notice if applicable */}
      {isUnverified && (
        <div className="mt-1.5 flex items-center gap-1 font-medium text-amber-800 dark:text-amber-300">
          <Info className="h-3.5 w-3.5 shrink-0" aria-hidden="true" />
          <span>Allergen information unverified</span>
        </div>
      )}

      {/* Specific containing allergens */}
      {hasContaining && (
        <div className="mt-2 text-xs">
          <span className="font-medium text-foreground">Listed as containing: </span>
          <span className="text-foreground/80">{containingAllergens.join(", ")}</span>
        </div>
      )}

      {/* Specific non-containing allergens */}
      {hasNotContaining && (
        <div className="mt-1 text-xs">
          <span className="font-medium text-foreground">Listed as not containing: </span>
          <span className="text-foreground/80">{notContainingAllergens.join(", ")}</span>
        </div>
      )}

      {/* Verbatim server notice */}
      {notice && notice.trim() && (
        <p className="mt-2 text-[11px] leading-relaxed text-muted-foreground border-t border-amber-500/20 pt-1.5">
          {notice.trim()}
        </p>
      )}
    </div>
  )
}
