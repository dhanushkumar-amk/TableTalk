import { UserCheck, ShieldAlert, Heart, Armchair, FileText } from "lucide-react"
import type { GetProfileOutput, EnableMemoryOutput } from "../types"
import { SpokenCaption } from "./SpokenCaption"

interface ProfileCardProps {
  profile?: GetProfileOutput | null
  enableMemory?: EnableMemoryOutput | null
  spokenText?: string | null
}

/**
 * ProfileCard renders a diner's stored profile preferences and privacy status.
 *
 * PRIVACY & SECURITY INVARIANTS:
 * - Profile bearer token is NEVER rendered in the DOM, logged to the console,
 *   or stored in localStorage.
 * - Stored user notes (user_provided_note or note preferences) are rendered strictly
 *   as plain escaped text inside a clearly labeled "Your note" block.
 *   They are never treated as instructions or parsed as HTML / markdown.
 */
export function ProfileCard({ profile, enableMemory, spokenText }: ProfileCardProps) {
  const guestName = profile?.guest_name || enableMemory?.guest_name || "Valued Diner"
  const preferences = profile?.preferences || []

  // Group preferences safely
  const allergies = preferences.filter((p) => p.kind === "allergy")
  const dietary = preferences.filter((p) => p.kind === "dietary")
  const seating = preferences.filter((p) => p.kind === "seat_pref")
  const favorites = preferences.filter((p) => p.kind === "favorite_item")
  const notes = preferences.filter((p) => p.kind === "note")

  return (
    <article
      aria-label={`Profile memory for ${guestName}`}
      className="flex flex-col space-y-4 rounded-2xl border border-border bg-card p-5 shadow-sm transition-all select-text"
    >
      {/* Header */}
      <header className="flex flex-wrap items-center justify-between gap-2 border-b border-border/60 pb-3">
        <div className="flex items-center gap-2">
          <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-primary/10 text-primary">
            <UserCheck className="h-4 w-4" aria-hidden="true" />
          </div>
          <div>
            <h2 className="text-sm font-semibold text-foreground">{guestName}</h2>
            <p className="text-xs text-muted-foreground">Opt-in Profile Memory</p>
          </div>
        </div>

        {/* Consent Badge */}
        <span className="rounded-full bg-emerald-500/15 px-2.5 py-0.5 text-xs font-semibold text-emerald-700 dark:text-emerald-300">
          Consent Active
        </span>
      </header>

      {/* Preferences Grid */}
      <div className="space-y-3">
        {/* Allergies Block */}
        {allergies.length > 0 && (
          <div className="rounded-xl border border-amber-500/30 bg-amber-500/5 p-3 space-y-1.5">
            <div className="flex items-center gap-1.5 text-xs font-semibold text-amber-800 dark:text-amber-400">
              <ShieldAlert className="h-3.5 w-3.5" aria-hidden="true" />
              <span>Known Allergies</span>
            </div>
            <div className="flex flex-wrap gap-1.5 pt-0.5">
              {allergies.map((a) => (
                <span
                  key={a.id}
                  className="rounded-md bg-amber-500/15 px-2 py-0.5 text-[11px] font-medium text-amber-900 dark:text-amber-200"
                >
                  {a.value}
                </span>
              ))}
            </div>
          </div>
        )}

        {/* Dietary Preferences */}
        {dietary.length > 0 && (
          <div className="rounded-xl border border-border/70 bg-muted/20 p-3 space-y-1.5">
            <div className="text-xs font-semibold text-foreground">Dietary Preferences</div>
            <div className="flex flex-wrap gap-1.5 pt-0.5">
              {dietary.map((d) => (
                <span
                  key={d.id}
                  className="rounded-md bg-secondary px-2 py-0.5 text-[11px] font-medium text-secondary-foreground"
                >
                  {d.value.replace("_", " ")}
                </span>
              ))}
            </div>
          </div>
        )}

        {/* Seating Preferences */}
        {seating.length > 0 && (
          <div className="rounded-xl border border-border/70 bg-muted/20 p-3 space-y-1.5">
            <div className="flex items-center gap-1.5 text-xs font-semibold text-foreground">
              <Armchair className="h-3.5 w-3.5 text-muted-foreground" aria-hidden="true" />
              <span>Seating Preference</span>
            </div>
            <div className="flex flex-wrap gap-1.5 pt-0.5">
              {seating.map((s) => (
                <span
                  key={s.id}
                  className="rounded-md bg-accent/40 px-2 py-0.5 text-[11px] font-medium text-accent-foreground capitalize"
                >
                  {s.value}
                </span>
              ))}
            </div>
          </div>
        )}

        {/* Favorite Dishes */}
        {favorites.length > 0 && (
          <div className="rounded-xl border border-border/70 bg-muted/20 p-3 space-y-1.5">
            <div className="flex items-center gap-1.5 text-xs font-semibold text-foreground">
              <Heart className="h-3.5 w-3.5 text-rose-500" aria-hidden="true" />
              <span>Favorite Dishes</span>
            </div>
            <div className="flex flex-wrap gap-1.5 pt-0.5">
              {favorites.map((f) => (
                <span
                  key={f.id}
                  className="rounded-md bg-secondary px-2 py-0.5 text-[11px] font-medium text-foreground"
                >
                  {f.value}
                </span>
              ))}
            </div>
          </div>
        )}

        {/* Stored User Note Block (Strict Inert Escaped Text) */}
        {notes.length > 0 && (
          <div className="rounded-xl border border-border/80 bg-muted/40 p-3 space-y-1.5">
            <div className="flex items-center gap-1.5 text-xs font-semibold text-foreground">
              <FileText className="h-3.5 w-3.5 text-muted-foreground" aria-hidden="true" />
              <span>Your note</span>
            </div>
            {notes.map((n) => (
              <p
                key={n.id}
                className="text-xs leading-relaxed text-foreground/90 font-mono whitespace-pre-wrap select-text break-words bg-background/60 p-2 rounded-md border border-border/40"
              >
                {String(n.value)}
              </p>
            ))}
          </div>
        )}
      </div>

      {/* Counts / Stats */}
      {profile?.counts && (
        <div className="border-t border-border/50 pt-2 text-[11px] text-muted-foreground flex items-center justify-between">
          <span>{profile.counts.preference_count} saved preferences</span>
          <span>
            {profile.counts.total_bookings} bookings • {profile.counts.total_orders} orders
          </span>
        </div>
      )}

      {/* Secondary Transcript Caption */}
      <SpokenCaption text={spokenText} />
    </article>
  )
}
