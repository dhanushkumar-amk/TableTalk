import { useState } from "react"
import { Link } from "react-router-dom"
import { motion, useReducedMotion } from "framer-motion"
import {
  MessageSquare,
  UtensilsCrossed,
  ArrowRight,
  Mic,
  Sparkles,
  CalendarCheck,
  Clock,
  AlertCircle,
  ShoppingBag,
  CreditCard,
  Users,
  Lock,
  ShieldCheck,
  Check,
  Terminal,
  Volume2,
  ChevronRight,
  Zap,
} from "lucide-react"
import { Button } from "@/components/ui/button"
import { StatusBadge } from "@/components/primitives/StatusBadge"
import { CodeBlock } from "@/components/primitives/CodeBlock"

interface Scenario {
  id: string
  label: string
  userText: string
  assistantText: string
  badgeLabel: string
  detailTitle: string
  detailSub: string
  code: string
}

const SCENARIOS: Scenario[] = [
  {
    id: "booking",
    label: "Reserve Table",
    userText: "Can we get a table for four this Friday around 7:30 PM? We'd love the patio.",
    assistantText: "I found an open table for four on the patio this Friday, October 16 at 7:30 PM. I've confirmed your reservation.",
    badgeLabel: "Confirmed",
    detailTitle: "Table for 4 · Patio Area",
    detailSub: "Fri, Oct 16 · 7:30 PM · Vikram Malhotra",
    code: "TT-849XQ",
  },
  {
    id: "allergen",
    label: "Allergen Audit",
    userText: "Which signature curries can we safely order with a severe tree nut allergy?",
    assistantText: "Dal Makhani and Chole contain zero tree nut ingredients. However, cross-contact risk is actively flagged.",
    badgeLabel: "Risk Notice",
    detailTitle: "Nut Allergen Disclosure",
    detailSub: "Kitchen Protocol #AL-204 · Cross-contact verified",
    code: "AL-SAFE-04",
  },
  {
    id: "pickup",
    label: "Pickup Order",
    userText: "Quick pickup: 2 Butter Chicken and 1 Garlic Naan for 8:15 PM.",
    assistantText: "Order confirmed! 2x Old Delhi Butter Chicken + 1x Garlic Naan. Total: ₹1,690. Ready in 22 min.",
    badgeLabel: "In Prep",
    detailTitle: "Pickup Order · Curbside Express",
    detailSub: "Ready at 8:15 PM · Counter Station #2",
    code: "PK-7749",
  },
]

export function HomePage() {
  const shouldReduceMotion = useReducedMotion()
  const [activeScenario, setActiveScenario] = useState<Scenario>(SCENARIOS[0])

  const containerVariants = {
    hidden: { opacity: 0 },
    visible: {
      opacity: 1,
      transition: { staggerChildren: shouldReduceMotion ? 0 : 0.08 },
    },
  }

  const itemVariants = {
    hidden: shouldReduceMotion ? { opacity: 1 } : { opacity: 0, y: 16 },
    visible: {
      opacity: 1,
      y: 0,
      transition: { duration: 0.35, ease: "easeOut" as const },
    },
  }

  return (
    <div className="relative min-h-screen bg-background selection:bg-primary/20 selection:text-foreground">
      {/* Subtle YC / Vercel subtle grid lines backdrop */}
      <div
        className="pointer-events-none absolute inset-0 -z-10 opacity-30 dark:opacity-20"
        style={{
          backgroundImage: "radial-gradient(circle, currentColor 1px, transparent 1px)",
          backgroundSize: "28px 28px",
          color: "var(--border)",
        }}
        aria-hidden="true"
      />

      {/* Top Ambient Glow */}
      <div
        className="pointer-events-none absolute -top-48 left-1/2 -z-10 h-[560px] w-[1000px] -translate-x-1/2 rounded-full bg-gradient-to-b from-primary/15 via-primary/5 to-transparent blur-3xl opacity-60"
        aria-hidden="true"
      />

      <div className="max-w-6xl mx-auto px-4 sm:px-6 lg:px-8 pt-8 sm:pt-14 pb-24">
        {/* ========================================================= */}
        {/* 1. YC HERO: HIGH-STATUS KICKER & VALUE PROPOSITION        */}
        {/* ========================================================= */}
        <section className="text-center max-w-3xl mx-auto space-y-6 pt-4 pb-14 sm:pb-20">
          {/* Status Badge Pill */}
          <motion.div
            initial={shouldReduceMotion ? { opacity: 1 } : { opacity: 0, y: -10 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.3 }}
            className="inline-flex items-center gap-2 rounded-full border border-border/80 bg-muted/50 px-3.5 py-1 text-xs font-mono font-medium text-foreground backdrop-blur-xs hover:border-primary/40 transition-colors shadow-2xs"
          >
            <span className="flex size-2 rounded-full bg-primary animate-pulse" />
            <span className="text-primary font-semibold">TableTalk 2.0</span>
            <span className="text-muted-foreground">·</span>
            <span className="text-muted-foreground">Model Context Protocol for Voice</span>
            <ChevronRight className="size-3 text-muted-foreground" />
          </motion.div>

          {/* Core Hero Headline */}
          <motion.h1
            initial={shouldReduceMotion ? { opacity: 1 } : { opacity: 0, y: 12 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.4, delay: 0.05 }}
            className="text-4xl sm:text-6xl lg:text-7xl font-bold tracking-tight text-foreground leading-[1.06]"
          >
            Book a table by just asking.
          </motion.h1>

          {/* Subheadline with Required Precision */}
          <motion.p
            initial={shouldReduceMotion ? { opacity: 1 } : { opacity: 0, y: 12 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.4, delay: 0.1 }}
            className="text-base sm:text-xl text-muted-foreground leading-relaxed max-w-2xl mx-auto font-normal"
          >
            An intelligent dining assistant that checks live table capacity, negotiates alternatives, and manages pickup orders naturally — powered by the Model Context Protocol.
          </motion.p>

          {/* Action CTAs */}
          <motion.div
            initial={shouldReduceMotion ? { opacity: 1 } : { opacity: 0, y: 12 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.4, delay: 0.15 }}
            className="flex flex-wrap items-center justify-center gap-3 pt-2"
          >
            <Button asChild size="lg" className="h-12 px-7 text-sm font-semibold shadow-sm group">
              <Link to="/assistant">
                <Mic className="size-4 mr-2 group-hover:scale-110 transition-transform" />
                Talk to the assistant
                <ArrowRight className="size-4 ml-2 group-hover:translate-x-1 transition-transform" />
              </Link>
            </Button>

            <Button asChild variant="outline" size="lg" className="h-12 px-6 text-sm font-semibold border-border/80">
              <Link to="/menu">
                <UtensilsCrossed className="size-4 mr-2" />
                View menu
              </Link>
            </Button>
          </motion.div>

          {/* Micro Trust Strip Below Hero CTAs */}
          <motion.div
            initial={shouldReduceMotion ? { opacity: 1 } : { opacity: 0 }}
            animate={{ opacity: 1 }}
            transition={{ duration: 0.4, delay: 0.2 }}
            className="flex flex-wrap items-center justify-center gap-x-8 gap-y-2 pt-4 text-xs font-mono text-muted-foreground"
          >
            <div className="flex items-center gap-1.5">
              <Check className="size-3.5 text-primary" />
              <span>Zero login required</span>
            </div>
            <div className="flex items-center gap-1.5">
              <Check className="size-3.5 text-primary" />
              <span>Sub-30ms search latency</span>
            </div>
            <div className="flex items-center gap-1.5">
              <Check className="size-3.5 text-primary" />
              <span>Simulated checkout</span>
            </div>
          </motion.div>
        </section>

        {/* ========================================================= */}
        {/* 2. THE HERO ARTIFACT: INTERACTIVE PRODUCT TERMINAL        */}
        {/* ========================================================= */}
        <section className="max-w-4xl mx-auto mb-20 sm:mb-28">
          <motion.div
            initial={shouldReduceMotion ? { opacity: 1 } : { opacity: 0, y: 24 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.5, delay: 0.25 }}
            className="rounded-2xl border border-border/80 bg-card shadow-2xl overflow-hidden transition-all"
          >
            {/* Terminal Window Header Bar */}
            <div className="flex items-center justify-between px-4 py-3 border-b border-border/70 bg-muted/40">
              <div className="flex items-center gap-2">
                <div className="flex items-center gap-1.5">
                  <span className="size-3 rounded-full bg-red-500/80" />
                  <span className="size-3 rounded-full bg-amber-500/80" />
                  <span className="size-3 rounded-full bg-emerald-500/80" />
                </div>
                <span className="text-xs font-mono text-muted-foreground ml-2 hidden sm:inline">
                  tabletalk-mcp-engine · 16kHz audio stream
                </span>
              </div>

              {/* Live Audio Visualizer Equalizer */}
              <div className="flex items-center gap-2 px-2.5 py-1 rounded-full bg-background border border-border/70 text-xs font-mono">
                <Volume2 className="size-3 text-primary animate-pulse" />
                <span className="text-[11px] text-muted-foreground">Voice Session Active</span>
                <span className="size-1.5 rounded-full bg-emerald-500 animate-ping ml-1" />
              </div>
            </div>

            {/* Interactive Scenario Tabs */}
            <div className="flex items-center gap-2 px-4 py-2.5 border-b border-border/50 bg-muted/10 overflow-x-auto no-scrollbar">
              <span className="text-[11px] font-mono uppercase text-muted-foreground shrink-0 mr-1">
                Prompt Scenarios:
              </span>
              {SCENARIOS.map((s) => (
                <button
                  key={s.id}
                  onClick={() => setActiveScenario(s)}
                  className={`text-xs px-3 py-1 rounded-md font-mono transition-all cursor-pointer ${
                    activeScenario.id === s.id
                      ? "bg-primary text-primary-foreground font-semibold shadow-2xs"
                      : "text-muted-foreground hover:text-foreground hover:bg-muted/60"
                  }`}
                >
                  {s.label}
                </button>
              ))}
            </div>

            {/* Terminal Conversation & Rendered Typed Card */}
            <div className="p-5 sm:p-7 space-y-4">
              {/* User Speech Bubble */}
              <div className="flex justify-end">
                <div className="max-w-[85%] rounded-2xl rounded-tr-xs bg-primary px-4 py-2.5 text-xs sm:text-sm text-primary-foreground font-medium shadow-xs">
                  {activeScenario.userText}
                </div>
              </div>

              {/* Assistant Speech Bubble */}
              <div className="flex justify-start">
                <div className="max-w-[90%] rounded-2xl rounded-tl-xs border border-border/80 bg-muted/40 px-4 py-2.5 text-xs sm:text-sm text-foreground leading-relaxed">
                  {activeScenario.assistantText}
                </div>
              </div>

              {/* Rendered Live Typed MCP Card */}
              <div className="pt-2">
                <div className="rounded-xl border border-border/90 bg-background p-4 sm:p-5 shadow-xs space-y-3">
                  <div className="flex items-start justify-between gap-2">
                    <div>
                      <div className="text-[11px] font-mono font-semibold uppercase tracking-wider text-primary">
                        MCP Tool Execution · Confirmed
                      </div>
                      <h4 className="text-base font-semibold text-foreground tracking-tight mt-0.5">
                        {activeScenario.detailTitle}
                      </h4>
                    </div>
                    <StatusBadge status="success" label={activeScenario.badgeLabel} size="sm" />
                  </div>

                  <div className="grid grid-cols-2 gap-2 text-xs pt-1 border-t border-border/60">
                    <div>
                      <span className="text-muted-foreground block text-[10px] uppercase font-mono">Date & Details</span>
                      <span className="font-semibold text-foreground">{activeScenario.detailSub}</span>
                    </div>
                    <div>
                      <span className="text-muted-foreground block text-[10px] uppercase font-mono">Status</span>
                      <span className="font-semibold text-foreground">Verified · The Saffron Nebula</span>
                    </div>
                  </div>

                  <div className="pt-2 flex items-center justify-between border-t border-border/60">
                    <CodeBlock code={activeScenario.code} label="Confirmation" />
                    <span className="text-[11px] font-mono text-muted-foreground">15 min grace period</span>
                  </div>
                </div>
              </div>
            </div>
          </motion.div>
        </section>

        {/* ========================================================= */}
        {/* 3. LOGO CLOUD / TECHNICAL METRICS TICKER                  */}
        {/* ========================================================= */}
        <section className="border-y border-border/70 py-6 mb-20 sm:mb-28">
          <div className="flex flex-wrap items-center justify-between gap-6 text-xs font-mono text-muted-foreground">
            <span className="font-semibold uppercase tracking-wider text-foreground">ARCHITECTURE</span>
            <div className="flex items-center gap-2">
              <Zap className="size-3.5 text-primary" />
              <span>24 RUST MCP TOOLS</span>
            </div>
            <div className="flex items-center gap-2">
              <Clock className="size-3.5 text-primary" />
              <span>SUB-30MS MEDIAN LATENCY</span>
            </div>
            <div className="flex items-center gap-2">
              <Terminal className="size-3.5 text-primary" />
              <span>SQLITE WAL CONCURRENCY</span>
            </div>
            <div className="flex items-center gap-2">
              <ShieldCheck className="size-3.5 text-primary" />
              <span>WCAG 2.2 AA VERIFIED</span>
            </div>
          </div>
        </section>

        {/* ========================================================= */}
        {/* 4. HOW IT WORKS (3 LINEAR STEPS)                          */}
        {/* ========================================================= */}
        <section className="mb-20 sm:mb-28 space-y-12">
          <div className="text-center max-w-xl mx-auto space-y-2">
            <h2 className="text-xs font-mono font-semibold uppercase tracking-wider text-primary">
              How it works
            </h2>
            <p className="text-3xl sm:text-4xl font-bold tracking-tight text-foreground">
              From natural conversation to confirmed table
            </p>
          </div>

          <motion.div
            variants={containerVariants}
            initial="hidden"
            whileInView="visible"
            viewport={{ once: true, margin: "-40px" }}
            className="grid grid-cols-1 md:grid-cols-3 gap-6"
          >
            {/* Step 1: Ask */}
            <motion.div
              variants={itemVariants}
              whileHover={shouldReduceMotion ? {} : { y: -4 }}
              className="rounded-2xl border border-border/80 bg-card p-6 space-y-3 shadow-xs hover:border-primary/50 transition-all"
            >
              <div className="flex size-10 items-center justify-center rounded-xl bg-primary/10 text-primary border border-primary/20">
                <MessageSquare className="size-5" />
              </div>
              <div className="text-xs font-mono font-semibold uppercase tracking-wider text-muted-foreground">
                Step 01
              </div>
              <h3 className="text-lg font-semibold text-foreground tracking-tight">Ask naturally</h3>
              <p className="text-sm text-muted-foreground leading-relaxed">
                State your party size, preferred time, and dietary requirements in normal everyday speech or text.
              </p>
            </motion.div>

            {/* Step 2: Choose */}
            <motion.div
              variants={itemVariants}
              whileHover={shouldReduceMotion ? {} : { y: -4 }}
              className="rounded-2xl border border-border/80 bg-card p-6 space-y-3 shadow-xs hover:border-primary/50 transition-all"
            >
              <div className="flex size-10 items-center justify-center rounded-xl bg-primary/10 text-primary border border-primary/20">
                <Sparkles className="size-5" />
              </div>
              <div className="text-xs font-mono font-semibold uppercase tracking-wider text-muted-foreground">
                Step 02
              </div>
              <h3 className="text-lg font-semibold text-foreground tracking-tight">Explore options</h3>
              <p className="text-sm text-muted-foreground leading-relaxed">
                Get instant availability snapshots or intelligent ranked alternatives when popular times are booked.
              </p>
            </motion.div>

            {/* Step 3: Confirm */}
            <motion.div
              variants={itemVariants}
              whileHover={shouldReduceMotion ? {} : { y: -4 }}
              className="rounded-2xl border border-border/80 bg-card p-6 space-y-3 shadow-xs hover:border-primary/50 transition-all"
            >
              <div className="flex size-10 items-center justify-center rounded-xl bg-primary/10 text-primary border border-primary/20">
                <CalendarCheck className="size-5" />
              </div>
              <div className="text-xs font-mono font-semibold uppercase tracking-wider text-muted-foreground">
                Step 03
              </div>
              <h3 className="text-lg font-semibold text-foreground tracking-tight">Confirm instantly</h3>
              <p className="text-sm text-muted-foreground leading-relaxed">
                Receive visual cards with confirmation codes, calendar downloads, and complete management controls.
              </p>
            </motion.div>
          </motion.div>
        </section>

        {/* ========================================================= */}
        {/* 5. YC BENTO FEATURE GRID (6 CELLS)                        */}
        {/* ========================================================= */}
        <section className="mb-20 sm:mb-28 space-y-12">
          <div className="text-center max-w-xl mx-auto space-y-2">
            <h2 className="text-xs font-mono font-semibold uppercase tracking-wider text-primary">
              Full Feature Suite
            </h2>
            <p className="text-3xl sm:text-4xl font-bold tracking-tight text-foreground">
              Production-grade hospitality intelligence
            </p>
          </div>

          <motion.div
            variants={containerVariants}
            initial="hidden"
            whileInView="visible"
            viewport={{ once: true, margin: "-40px" }}
            className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-5"
          >
            {/* Bento 1: Smart Alternatives */}
            <motion.div
              variants={itemVariants}
              whileHover={shouldReduceMotion ? {} : { y: -4 }}
              className="rounded-2xl border border-border/80 bg-card p-6 space-y-3 shadow-xs hover:border-primary/50 transition-all"
            >
              <div className="flex size-9 items-center justify-center rounded-lg bg-muted text-foreground">
                <Clock className="size-4 text-primary" />
              </div>
              <h3 className="text-base font-semibold text-foreground tracking-tight">
                Smart alternatives
              </h3>
              <p className="text-xs sm:text-sm text-muted-foreground leading-relaxed">
                Multi-criteria scoring discovers nearest times, alternative seating areas, and adjacent open dates.
              </p>
            </motion.div>

            {/* Bento 2: Allergen-Aware Menu */}
            <motion.div
              variants={itemVariants}
              whileHover={shouldReduceMotion ? {} : { y: -4 }}
              className="rounded-2xl border border-border/80 bg-card p-6 space-y-3 shadow-xs hover:border-primary/50 transition-all"
            >
              <div className="flex size-9 items-center justify-center rounded-lg bg-muted text-foreground">
                <AlertCircle className="size-4 text-primary" />
              </div>
              <h3 className="text-base font-semibold text-foreground tracking-tight">
                Allergen-aware menu
              </h3>
              <p className="text-xs sm:text-sm text-muted-foreground leading-relaxed">
                Strict disclosures with non-bypassable conflict blocks. Cross-contact risk is actively flagged, never assumed safe.
              </p>
            </motion.div>

            {/* Bento 3: Pickup Orders */}
            <motion.div
              variants={itemVariants}
              whileHover={shouldReduceMotion ? {} : { y: -4 }}
              className="rounded-2xl border border-border/80 bg-card p-6 space-y-3 shadow-xs hover:border-primary/50 transition-all"
            >
              <div className="flex size-9 items-center justify-center rounded-lg bg-muted text-foreground">
                <ShoppingBag className="size-4 text-primary" />
              </div>
              <h3 className="text-base font-semibold text-foreground tracking-tight">
                Pickup orders
              </h3>
              <p className="text-xs sm:text-sm text-muted-foreground leading-relaxed">
                Integer-precision pricing snapshots, dynamic prep time calculation, and curbside pickup codes.
              </p>
            </motion.div>

            {/* Bento 4: Deposits (Simulated) */}
            <motion.div
              variants={itemVariants}
              whileHover={shouldReduceMotion ? {} : { y: -4 }}
              className="rounded-2xl border border-border/80 bg-card p-6 space-y-3 shadow-xs hover:border-primary/50 transition-all"
            >
              <div className="flex size-9 items-center justify-center rounded-lg bg-muted text-foreground">
                <CreditCard className="size-4 text-primary" />
              </div>
              <h3 className="text-base font-semibold text-foreground tracking-tight">
                Deposits (simulated)
              </h3>
              <p className="text-xs sm:text-sm text-muted-foreground leading-relaxed">
                Demonstrates group deposit holds and strict countdown ledgers without touching real money.
              </p>
            </motion.div>

            {/* Bento 5: Waitlist */}
            <motion.div
              variants={itemVariants}
              whileHover={shouldReduceMotion ? {} : { y: -4 }}
              className="rounded-2xl border border-border/80 bg-card p-6 space-y-3 shadow-xs hover:border-primary/50 transition-all"
            >
              <div className="flex size-9 items-center justify-center rounded-lg bg-muted text-foreground">
                <Users className="size-4 text-primary" />
              </div>
              <h3 className="text-base font-semibold text-foreground tracking-tight">
                Fair FIFO waitlist
              </h3>
              <p className="text-xs sm:text-sm text-muted-foreground leading-relaxed">
                Queues parties automatically when tables fill up. Claim cancellations with cryptographic tokens.
              </p>
            </motion.div>

            {/* Bento 6: Privacy-First Memory */}
            <motion.div
              variants={itemVariants}
              whileHover={shouldReduceMotion ? {} : { y: -4 }}
              className="rounded-2xl border border-border/80 bg-card p-6 space-y-3 shadow-xs hover:border-primary/50 transition-all"
            >
              <div className="flex size-9 items-center justify-center rounded-lg bg-muted text-foreground">
                <Lock className="size-4 text-primary" />
              </div>
              <h3 className="text-base font-semibold text-foreground tracking-tight">
                Privacy-first memory
              </h3>
              <p className="text-xs sm:text-sm text-muted-foreground leading-relaxed">
                Explicit opt-in memory with typed preferences. Export or permanently erase your history anytime.
              </p>
            </motion.div>
          </motion.div>
        </section>

        {/* ========================================================= */}
        {/* 6. TRUST & SAFETY STRIP                                   */}
        {/* ========================================================= */}
        <section className="mb-20 sm:mb-28">
          <div className="flex flex-col sm:flex-row items-center justify-between gap-4 rounded-2xl border border-border/80 bg-muted/30 px-6 py-4 text-xs text-muted-foreground">
            <div className="flex items-center gap-2.5 text-foreground font-medium">
              <ShieldCheck className="size-4 text-primary shrink-0" />
              <span>Demo project. No real payments. Your data can be deleted any time.</span>
            </div>
            <div className="flex items-center gap-4 shrink-0 font-mono">
              <Link to="/privacy" className="hover:text-foreground underline underline-offset-4 transition-colors">
                Privacy Policy
              </Link>
              <span className="text-border">·</span>
              <span>Zero telemetry</span>
            </div>
          </div>
        </section>

        {/* ========================================================= */}
        {/* 7. GRAND CLOSING HERO                                     */}
        {/* ========================================================= */}
        <section className="text-center mb-20 sm:mb-24">
          <motion.div
            initial={shouldReduceMotion ? { opacity: 1 } : { opacity: 0, scale: 0.98 }}
            whileInView={{ opacity: 1, scale: 1 }}
            viewport={{ once: true, margin: "-40px" }}
            transition={{ duration: 0.5 }}
            className="rounded-3xl border border-border/90 bg-gradient-to-b from-card to-muted/20 p-8 sm:p-14 space-y-6 max-w-3xl mx-auto shadow-sm"
          >
            <h2 className="text-3xl sm:text-5xl font-bold tracking-tight text-foreground">
              Experience the assistant now
            </h2>
            <p className="text-sm sm:text-base text-muted-foreground max-w-lg mx-auto leading-relaxed">
              Launch the interactive voice interface to explore availability, negotiate reservations, or simulate pickup orders.
            </p>
            <div className="pt-2 flex justify-center">
              <Button asChild size="lg" className="h-12 px-8 text-sm font-semibold shadow-sm">
                <Link to="/assistant">
                  <Mic className="size-4 mr-2" />
                  Talk to the assistant
                  <ArrowRight className="size-4 ml-2" />
                </Link>
              </Button>
            </div>
          </motion.div>
        </section>

        {/* ========================================================= */}
        {/* 8. YC STARTUP MINIMALIST FOOTER                           */}
        {/* ========================================================= */}
        <footer className="pt-8 border-t border-border/60 text-xs text-muted-foreground">
          <div className="flex flex-col sm:flex-row items-center justify-between gap-4 font-mono">
            <div className="flex items-center gap-2 font-medium text-foreground">
              <span className="font-bold">TableTalk</span>
              <span className="size-1 rounded-full bg-primary" />
              <span className="text-muted-foreground text-[11px] font-normal">
                Built for the Amazon Developer Hackathon (Alexa+ track)
              </span>
            </div>

            <nav className="flex items-center gap-5 text-xs" aria-label="Footer Navigation">
              <Link to="/assistant" className="hover:text-foreground transition-colors">Assistant</Link>
              <Link to="/menu" className="hover:text-foreground transition-colors">Menu</Link>
              <Link to="/reserve" className="hover:text-foreground transition-colors">Reserve</Link>
              <Link to="/order" className="hover:text-foreground transition-colors">Order</Link>
              <Link to="/manage" className="hover:text-foreground transition-colors">Manage</Link>
              <Link to="/privacy" className="hover:text-foreground transition-colors">Privacy</Link>
            </nav>
          </div>
        </footer>
      </div>
    </div>
  )
}
