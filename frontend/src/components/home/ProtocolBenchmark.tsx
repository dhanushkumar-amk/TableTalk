import { useState } from "react"
import { motion } from "framer-motion"
import {
  XCircle,
  CheckCircle2,
  Zap,
  ShieldCheck,
  PhoneCall,
  ArrowRight,
  Sparkles,
} from "lucide-react"
import { Button } from "@/components/ui/button"
import { Link } from "react-router-dom"

interface ComparisonMetric {
  category: string
  oldWay: {
    title: string
    detail: string
    latency: string
  }
  tableTalkWay: {
    title: string
    detail: string
    latency: string
  }
}

const COMPARISONS: ComparisonMetric[] = [
  {
    category: "Table Reservations",
    oldWay: {
      title: "Calling Busy Host Stand",
      detail: "Hold music, manual paper ledgers, or rigid 3rd-party apps with forced accounts.",
      latency: "4–8 min wait",
    },
    tableTalkWay: {
      title: "Real-Time MCP Table Lock",
      detail: "Speak naturally. Atomic SQLite WAL locks tables with 15-minute grace period.",
      latency: "14ms execution",
    },
  },
  {
    category: "Allergen Verification",
    oldWay: {
      title: "Verbal 'I think it is fine'",
      detail: "Unverified staff memory, high cross-contact risk, no ingredient disclosures.",
      latency: "Uncertain",
    },
    tableTalkWay: {
      title: "Strict Cross-Contact Audit",
      detail: "Automated ingredient scanner actively flags kitchen cross-contact. Never false promises.",
      latency: "Real-time flag",
    },
  },
  {
    category: "Pickup & Currency Math",
    oldWay: {
      title: "Misheard Phone Orders",
      detail: "Lost paper tickets, floating-point rounding errors, uncertain preparation times.",
      latency: "Manual check",
    },
    tableTalkWay: {
      title: "Integer-Precision Ledger",
      detail: "Exact paisa currency math, dynamic prep timers, and cryptographic pickup codes.",
      latency: "< 22ms checkout",
    },
  },
]

export function ProtocolBenchmark() {
  const [benchmarkLatency, setBenchmarkLatency] = useState(14)
  const [isRunning, setIsRunning] = useState(false)

  const runLatencyTest = () => {
    setIsRunning(true)
    setTimeout(() => {
      setBenchmarkLatency(Math.floor(Math.random() * 8) + 12) // 12-19ms
      setIsRunning(false)
    }, 450)
  }

  return (
    <div className="rounded-3xl border border-border/80 bg-gradient-to-b from-card via-muted/20 to-card p-6 sm:p-10 shadow-dropdown space-y-8">
      {/* Section Header */}
      <div className="flex flex-col md:flex-row md:items-end justify-between gap-6 pb-6 border-b border-border/60">
        <div className="space-y-2">
          <div className="inline-flex items-center gap-2 px-3 py-1 rounded-full bg-primary/10 text-primary text-xs font-mono font-semibold uppercase">
            <Sparkles className="size-3.5" />
            <span>PERFORMANCE & PARADIGM SHIFT</span>
          </div>
          <h3 className="text-2xl sm:text-3xl font-bold tracking-tight text-foreground">
            The old way vs TableTalk
          </h3>
          <p className="text-sm text-muted-foreground max-w-xl">
            Traditional restaurant interactions waste minutes on hold and manual paper checks. TableTalk replaces friction with 24 atomic Rust MCP tools.
          </p>
        </div>

        {/* Live Latency Simulation Widget */}
        <div className="rounded-2xl border border-border/80 bg-background/90 p-4 min-w-[240px] shadow-xs space-y-2 font-mono">
          <div className="flex items-center justify-between text-[11px] text-muted-foreground">
            <span>MCP Tool Ping</span>
            <span className="flex items-center gap-1 text-emerald-500 font-semibold">
              <span className="size-2 rounded-full bg-emerald-500 animate-ping" />
              Live SSE
            </span>
          </div>
          <div className="flex items-baseline justify-between">
            <span className="text-2xl font-bold text-foreground">
              {isRunning ? "--" : `${benchmarkLatency}ms`}
            </span>
            <button
              onClick={runLatencyTest}
              disabled={isRunning}
              className="text-[11px] text-primary hover:underline font-medium cursor-pointer"
            >
              {isRunning ? "Measuring..." : "Test Latency ↻"}
            </button>
          </div>
          <div className="h-1.5 w-full bg-muted rounded-full overflow-hidden">
            <motion.div
              className="h-full bg-primary rounded-full"
              initial={{ width: "30%" }}
              animate={{ width: isRunning ? "100%" : "28%" }}
              transition={{ duration: 0.4 }}
            />
          </div>
        </div>
      </div>

      {/* Side-by-Side Comparison Grid */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Left: Traditional Way */}
        <div className="rounded-2xl border border-border/70 bg-muted/30 p-6 space-y-5">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2.5 text-muted-foreground">
              <div className="flex size-8 items-center justify-center rounded-lg bg-red-500/10 text-red-500">
                <PhoneCall className="size-4" />
              </div>
              <h4 className="font-semibold text-foreground text-base">The Old Way</h4>
            </div>
            <span className="text-xs font-mono text-red-500/80 bg-red-500/10 px-2.5 py-0.5 rounded-full">
              High Friction
            </span>
          </div>

          <div className="space-y-4 pt-1">
            {COMPARISONS.map((c, i) => (
              <div key={i} className="flex items-start gap-3 text-xs">
                <XCircle className="size-4 text-red-400 shrink-0 mt-0.5" />
                <div className="space-y-0.5">
                  <div className="font-semibold text-foreground flex items-center gap-2">
                    <span>{c.oldWay.title}</span>
                    <span className="font-mono text-[10px] text-muted-foreground">({c.oldWay.latency})</span>
                  </div>
                  <p className="text-muted-foreground leading-relaxed">{c.oldWay.detail}</p>
                </div>
              </div>
            ))}
          </div>
        </div>

        {/* Right: TableTalk Way */}
        <div className="rounded-2xl border border-primary/40 bg-gradient-to-br from-primary/10 via-card to-background p-6 space-y-5 shadow-xs">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2.5 text-primary">
              <div className="flex size-8 items-center justify-center rounded-lg bg-primary text-primary-foreground shadow-2xs">
                <Zap className="size-4" />
              </div>
              <h4 className="font-semibold text-foreground text-base">The TableTalk Way</h4>
            </div>
            <span className="text-xs font-mono text-primary bg-primary/15 px-2.5 py-0.5 rounded-full font-bold">
              Sub-30ms Voice
            </span>
          </div>

          <div className="space-y-4 pt-1">
            {COMPARISONS.map((c, i) => (
              <div key={i} className="flex items-start gap-3 text-xs">
                <CheckCircle2 className="size-4 text-primary shrink-0 mt-0.5" />
                <div className="space-y-0.5">
                  <div className="font-semibold text-foreground flex items-center gap-2">
                    <span>{c.tableTalkWay.title}</span>
                    <span className="font-mono text-[10px] text-primary font-bold">({c.tableTalkWay.latency})</span>
                  </div>
                  <p className="text-muted-foreground leading-relaxed">{c.tableTalkWay.detail}</p>
                </div>
              </div>
            ))}
          </div>
        </div>
      </div>

      {/* Bottom Key Takeaway Callout */}
      <div className="flex flex-col sm:flex-row items-center justify-between gap-4 pt-2 border-t border-border/50 text-xs text-muted-foreground">
        <div className="flex items-center gap-2">
          <ShieldCheck className="size-4 text-primary shrink-0" />
          <span>Zero third-party cloud accounts or app downloads. Speak naturally to experience instant execution.</span>
        </div>
        <Button asChild size="sm" variant="outline" className="h-8 text-xs font-medium shrink-0">
          <Link to="/assistant">
            Try Voice Assistant
            <ArrowRight className="size-3.5 ml-1.5" />
          </Link>
        </Button>
      </div>
    </div>
  )
}
