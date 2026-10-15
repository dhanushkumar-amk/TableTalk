import { useState } from "react"
import { motion, useReducedMotion } from "framer-motion"
import { Mic, Volume2, AlertCircle, ShoppingBag } from "lucide-react"
import { StatusBadge } from "@/components/primitives/StatusBadge"
import { CodeBlock } from "@/components/primitives/CodeBlock"

interface PromptScenario {
  id: string
  label: string
  category: string
  userMessage: string
  assistantResponse: string
  cardType: "reservation" | "allergen" | "pickup"
}

const SCENARIOS: PromptScenario[] = [
  {
    id: "reservation",
    label: "Table for four on patio",
    category: "Reservation",
    userMessage: "Can we get a table for four this Friday around 7:30 PM? We'd love the patio.",
    assistantResponse:
      "I found an open table for four on the patio this Friday, October 16 at 7:30 PM. I've locked your reservation for Table #04.",
    cardType: "reservation",
  },
  {
    id: "allergen",
    label: "Nut-aware curry check",
    category: "Allergen Audit",
    userMessage: "Are your signature curries safe for someone with a severe cashew nut allergy?",
    assistantResponse:
      "Nebula Dal Makhani and Pindi Chole contain zero tree nut ingredients. However, our kitchen processes cashews; cross-contact risk is actively flagged.",
    cardType: "allergen",
  },
  {
    id: "pickup",
    label: "Quick dinner pickup",
    category: "Pickup Order",
    userMessage: "Can I place a pickup order for 2 Butter Chicken and 1 Garlic Naan for 8:15 PM?",
    assistantResponse:
      "Order created! 2x Old Delhi Butter Chicken + 1x Truffle Garlic Naan. Total is ₹1,690. Ready for curbside pickup in 22 minutes.",
    cardType: "pickup",
  },
]

export function LiveAudioVisualizer() {
  const shouldReduceMotion = useReducedMotion()
  const [activeScenario, setActiveScenario] = useState<PromptScenario>(SCENARIOS[0])
  const [isPlaying, setIsPlaying] = useState(true)
  const [step, setStep] = useState(3) // 0: reset, 1: user msg, 2: voice response, 3: card reveal

  const handleSelectScenario = (scenario: PromptScenario) => {
    setActiveScenario(scenario)
    if (shouldReduceMotion) {
      setStep(3)
      return
    }

    setStep(0)
    setIsPlaying(true)
    setTimeout(() => setStep(1), 250)
    setTimeout(() => setStep(2), 700)
    setTimeout(() => setStep(3), 1200)
  }

  // Audio equalizer bar heights
  const barHeights = [14, 22, 32, 18, 28, 36, 20, 12, 26, 34, 16, 30, 24, 14, 20]

  return (
    <div className="relative rounded-2xl border border-border/80 bg-card p-5 sm:p-6 shadow-dropdown transition-all">
      {/* Top Header with Live Indicator & Voice Engine Pill */}
      <div className="flex flex-wrap items-center justify-between gap-3 pb-4 mb-4 border-b border-border/60">
        <div className="flex items-center gap-2">
          <span className="relative flex size-2.5">
            <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-emerald-400 opacity-75" />
            <span className="relative inline-flex rounded-full size-2.5 bg-emerald-500" />
          </span>
          <span className="text-xs font-semibold text-foreground">The Saffron Nebula Bistro</span>
          <span className="text-[11px] text-muted-foreground font-mono">Live Voice Session</span>
        </div>

        {/* Audio Waveform Equalizer */}
        <div className="flex items-center gap-1.5 px-3 py-1 rounded-full bg-muted/60 border border-border/60">
          <Volume2 className="size-3.5 text-primary animate-pulse" />
          <div className="flex items-end gap-1 h-5 w-24">
            {barHeights.map((h, i) => (
              <span
                key={i}
                className="w-1 rounded-full bg-primary/70 transition-all duration-300"
                style={{
                  height: isPlaying ? `${Math.max(4, (h * (i % 2 === 0 ? 1 : 0.8)))}px` : "4px",
                  animation: !shouldReduceMotion && isPlaying ? `soundwaveBar 1.${(i % 5) + 2}s ease-in-out infinite` : "none",
                }}
              />
            ))}
          </div>
          <span className="text-[10px] font-mono text-muted-foreground ml-1">16kHz SSE</span>
        </div>
      </div>

      {/* Interactive Scenario Selector Pills */}
      <div className="mb-4">
        <div className="text-[11px] font-mono uppercase tracking-wider text-muted-foreground mb-2 flex items-center justify-between">
          <span>Try a live guest prompt:</span>
          <span className="text-[10px] text-primary">Click to test instant MCP card</span>
        </div>
        <div className="flex flex-wrap gap-2">
          {SCENARIOS.map((s) => (
            <button
              key={s.id}
              onClick={() => handleSelectScenario(s)}
              className={`text-xs px-3 py-1.5 rounded-lg border font-medium transition-all text-left flex items-center gap-1.5 ${
                activeScenario.id === s.id
                  ? "border-primary bg-primary/10 text-primary shadow-2xs"
                  : "border-border/70 bg-muted/30 text-muted-foreground hover:border-border hover:text-foreground"
              }`}
            >
              <Mic className="size-3 shrink-0" />
              <span>{s.label}</span>
            </button>
          ))}
        </div>
      </div>

      {/* Chat Thread */}
      <div className="space-y-3.5 min-h-[220px]">
        {/* User Message */}
        {step >= 1 && (
          <motion.div
            initial={shouldReduceMotion ? { opacity: 1 } : { opacity: 0, y: 8 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.2 }}
            className="flex justify-end"
          >
            <div className="max-w-[88%] rounded-2xl rounded-tr-xs bg-primary px-4 py-2.5 text-xs sm:text-sm text-primary-foreground leading-relaxed shadow-xs">
              {activeScenario.userMessage}
            </div>
          </motion.div>
        )}

        {/* Assistant Response */}
        {step >= 2 && (
          <motion.div
            initial={shouldReduceMotion ? { opacity: 1 } : { opacity: 0, y: 8 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.2 }}
            className="flex justify-start"
          >
            <div className="max-w-[92%] rounded-2xl rounded-tl-xs border border-border/80 bg-muted/40 px-4 py-2.5 text-xs sm:text-sm text-foreground leading-relaxed">
              {activeScenario.assistantResponse}
            </div>
          </motion.div>
        )}

        {/* Dynamic MCP Typed Card */}
        {step >= 3 && activeScenario.cardType === "reservation" && (
          <motion.div
            initial={shouldReduceMotion ? { opacity: 1 } : { opacity: 0, scale: 0.98 }}
            animate={{ opacity: 1, scale: 1 }}
            transition={{ duration: 0.25 }}
            className="rounded-xl border border-border/90 bg-background p-4 sm:p-5 shadow-xs space-y-3"
          >
            <div className="flex items-start justify-between gap-2">
              <div>
                <div className="text-[11px] font-semibold uppercase tracking-wider text-primary">
                  Confirmed Reservation
                </div>
                <h4 className="text-base font-semibold text-foreground tracking-tight mt-0.5">
                  Table for 4 · Patio Area
                </h4>
              </div>
              <StatusBadge status="success" label="Confirmed" size="sm" />
            </div>

            <div className="grid grid-cols-2 gap-2 text-xs pt-1 border-t border-border/60">
              <div>
                <span className="text-muted-foreground block text-[10px] uppercase font-medium">Date & Time</span>
                <span className="font-semibold text-foreground">Fri, Oct 16 · 7:30 PM</span>
              </div>
              <div>
                <span className="text-muted-foreground block text-[10px] uppercase font-medium">Guest Name</span>
                <span className="font-semibold text-foreground">Vikram Malhotra</span>
              </div>
            </div>

            <div className="pt-2 flex items-center justify-between border-t border-border/60">
              <CodeBlock code="TT-849XQ" label="Confirmation" />
              <span className="text-[11px] text-muted-foreground">15 min grace period</span>
            </div>
          </motion.div>
        )}

        {step >= 3 && activeScenario.cardType === "allergen" && (
          <motion.div
            initial={shouldReduceMotion ? { opacity: 1 } : { opacity: 0, scale: 0.98 }}
            animate={{ opacity: 1, scale: 1 }}
            transition={{ duration: 0.25 }}
            className="rounded-xl border border-amber-500/40 bg-amber-50/40 dark:bg-amber-950/20 p-4 sm:p-5 shadow-xs space-y-3"
          >
            <div className="flex items-start justify-between gap-2">
              <div className="flex items-center gap-2">
                <AlertCircle className="size-4 text-amber-600 dark:text-amber-400 shrink-0" />
                <div>
                  <div className="text-[11px] font-semibold uppercase tracking-wider text-amber-700 dark:text-amber-400">
                    Allergen Risk Disclosure
                  </div>
                  <h4 className="text-sm font-semibold text-foreground tracking-tight">
                    Tree Nut Interceptor
                  </h4>
                </div>
              </div>
              <StatusBadge status="warning" label="Cross-Contact Risk" size="sm" />
            </div>

            <p className="text-xs text-muted-foreground leading-relaxed">
              No cashew or almond paste in Dal Makhani or Chole. However, shared kitchen preparation means cross-contact risk is actively flagged.
            </p>

            <div className="pt-2 flex items-center justify-between border-t border-amber-500/20 text-xs">
              <span className="text-muted-foreground font-mono">Kitchen protocol #AL-204</span>
              <span className="font-medium text-amber-700 dark:text-amber-400">Verified by Chef</span>
            </div>
          </motion.div>
        )}

        {step >= 3 && activeScenario.cardType === "pickup" && (
          <motion.div
            initial={shouldReduceMotion ? { opacity: 1 } : { opacity: 0, scale: 0.98 }}
            animate={{ opacity: 1, scale: 1 }}
            transition={{ duration: 0.25 }}
            className="rounded-xl border border-border/90 bg-background p-4 sm:p-5 shadow-xs space-y-3"
          >
            <div className="flex items-start justify-between gap-2">
              <div className="flex items-center gap-2">
                <ShoppingBag className="size-4 text-primary shrink-0" />
                <div>
                  <div className="text-[11px] font-semibold uppercase tracking-wider text-primary">
                    Pickup Order Ready in 22m
                  </div>
                  <h4 className="text-sm font-semibold text-foreground tracking-tight">
                    2x Butter Chicken · 1x Garlic Naan
                  </h4>
                </div>
              </div>
              <StatusBadge status="accent" label="In Prep" size="sm" />
            </div>

            <div className="flex items-center justify-between text-xs pt-1 border-t border-border/60">
              <span className="text-muted-foreground">Order Total (Integer Precision)</span>
              <span className="font-semibold text-foreground font-mono">₹1,690.00</span>
            </div>

            <div className="pt-2 flex items-center justify-between border-t border-border/60">
              <CodeBlock code="PK-7749" label="Curbside Code" />
              <span className="text-[11px] text-muted-foreground">Express Counter Pickup</span>
            </div>
          </motion.div>
        )}
      </div>
    </div>
  )
}
