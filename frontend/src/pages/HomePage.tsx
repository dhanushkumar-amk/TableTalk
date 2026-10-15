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
  Flame,
  MapPin,
  ExternalLink,
  Radio,
  Coffee,
} from "lucide-react"
import { Button } from "@/components/ui/button"
import { BistroPanorama } from "@/components/home/BistroPanorama"
import { LiveAudioVisualizer } from "@/components/home/LiveAudioVisualizer"
import { EventHeroBanner } from "@/components/home/EventHeroBanner"

export function HomePage() {
  const shouldReduceMotion = useReducedMotion()
  const [activeTab, setActiveTab] = useState("overview")

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
      transition: { duration: 0.25, ease: "easeOut" as const },
    },
  }

  const scrollToSection = (id: string) => {
    setActiveTab(id)
    const element = document.getElementById(id)
    if (element) {
      element.scrollIntoView({ behavior: shouldReduceMotion ? "auto" : "smooth" })
    }
  }

  return (
    <div className="relative overflow-hidden">
      {/* Background radial gradient glow (warm terracotta & twilight atmosphere) */}
      <div
        className="pointer-events-none absolute -top-40 left-1/2 -z-10 h-[600px] w-[1000px] -translate-x-1/2 rounded-full bg-gradient-to-b from-primary/15 via-primary/5 to-transparent blur-3xl opacity-80 dark:opacity-40"
        aria-hidden="true"
      />

      <div className="max-w-6xl mx-auto px-4 sm:px-6 lg:px-8 pt-4 sm:pt-8 pb-20">
        {/* ========================================================= */}
        {/* TOP EVENT STICKER / BANNER (Inspired by Screenshot 1)     */}
        {/* ========================================================= */}
        <EventHeroBanner />

        {/* ========================================================= */}
        {/* 1. HERO SECTION                                           */}
        {/* ========================================================= */}
        <section className="grid grid-cols-1 lg:grid-cols-12 gap-10 lg:gap-8 items-start pt-2 sm:pt-4 pb-12 lg:pb-16">
          {/* Left Column: Headline & Value Proposition */}
          <div className="lg:col-span-6 space-y-6">
            <div className="inline-flex items-center gap-2 rounded-full border border-border/80 bg-muted/60 px-3.5 py-1 text-xs font-medium text-foreground/90 backdrop-blur-xs">
              <span className="flex size-2 rounded-full bg-primary animate-pulse" />
              <span className="font-mono uppercase tracking-wider text-[11px] text-muted-foreground">
                EVENT 02 · REAL-TIME MCP ASSISTANT
              </span>
            </div>

            <h1 className="text-4xl sm:text-5xl lg:text-6xl font-semibold tracking-tight text-foreground leading-[1.08]">
              Book a table by just asking.
            </h1>

            <p className="text-base sm:text-lg text-muted-foreground leading-relaxed max-w-xl">
              An intelligent dining assistant that checks live table capacity, negotiates alternatives, and manages pickup orders naturally — powered by the Model Context Protocol.
            </p>

            {/* Primary Action Buttons */}
            <div className="flex flex-wrap items-center gap-3 pt-2">
              <Button asChild size="lg" className="h-11 px-6 text-sm font-medium shadow-sm group">
                <Link to="/assistant">
                  <Mic className="size-4 mr-2 group-hover:scale-110 transition-transform" />
                  Talk to the assistant
                  <ArrowRight className="size-4 ml-2 group-hover:translate-x-0.5 transition-transform" />
                </Link>
              </Button>

              <Button asChild variant="outline" size="lg" className="h-11 px-5 text-sm font-medium">
                <Link to="/menu">
                  <UtensilsCrossed className="size-4 mr-2" />
                  View menu
                </Link>
              </Button>
            </div>

            {/* Key Trust & Performance Checkmarks */}
            <div className="flex flex-wrap items-center gap-x-6 gap-y-2 pt-4 text-xs text-muted-foreground border-t border-border/60">
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
            </div>

            {/* Live Event Strip (Screenshot 1 Style) */}
            <div className="rounded-xl border border-border/70 bg-muted/20 p-4 space-y-2.5 font-mono text-xs">
              <div className="flex items-center justify-between text-[11px] text-muted-foreground">
                <span className="uppercase tracking-wider">HYBRID WEEK · OCT 8 – 11, 2026</span>
                <span>TABLES OF 1 TO 8</span>
              </div>
              <div className="grid grid-cols-2 gap-3 pt-1 border-t border-border/50 text-[12px]">
                <div>
                  <span className="text-muted-foreground block text-[10px] uppercase font-semibold">ONLINE MCP CORE</span>
                  <span className="font-semibold text-foreground">Anywhere via HTTP / SSE</span>
                </div>
                <div>
                  <span className="text-muted-foreground block text-[10px] uppercase font-semibold">IN PERSON DINING</span>
                  <span className="font-semibold text-foreground">The Saffron Nebula Bistro</span>
                </div>
              </div>
            </div>
          </div>

          {/* Right Column: Live Audio Visualizer & Interactive Simulation */}
          <div className="lg:col-span-6 space-y-4">
            <LiveAudioVisualizer />

            {/* Quick Live Bistro Status Card (Inspired by Screenshot 1 Side Widget) */}
            <div className="rounded-xl border border-border/70 bg-muted/30 p-4 flex items-center justify-between text-xs">
              <div className="flex items-center gap-2.5">
                <span className="size-2 rounded-full bg-emerald-500 animate-pulse" />
                <div>
                  <span className="font-semibold text-foreground block">1,420 Sessions Served</span>
                  <span className="text-[11px] text-muted-foreground">Current wait time: 0 min (Direct Seating)</span>
                </div>
              </div>
              <Button asChild variant="ghost" size="sm" className="h-8 text-xs font-medium">
                <Link to="/reserve">
                  Reserve table
                  <ArrowRight className="size-3 ml-1.5" />
                </Link>
              </Button>
            </div>
          </div>
        </section>
      </div>

      {/* ========================================================= */}
      {/* 2. PANORAMIC SCENIC SKYLINE BANNER (Inspired by Screenshot 2) */}
      {/* ========================================================= */}
      <BistroPanorama />

      {/* ========================================================= */}
      {/* 3. IN-PAGE SUB-NAVIGATION BAR (Inspired by Screenshot 2)  */}
      {/* ========================================================= */}
      <div className="sticky top-16 z-30 w-full border-b border-border/80 bg-background/95 backdrop-blur-md">
        <div className="max-w-6xl mx-auto px-4 sm:px-6 lg:px-8">
          <nav className="flex items-center gap-1 sm:gap-4 overflow-x-auto no-scrollbar py-2 text-xs font-mono uppercase tracking-wider text-muted-foreground">
            {[
              { id: "overview", label: "01 / OVERVIEW" },
              { id: "how-it-works", label: "02 / HOW IT WORKS" },
              { id: "tracks", label: "03 / CAPABILITIES" },
              { id: "suite", label: "04 / FEATURE SUITE" },
              { id: "atmosphere", label: "05 / THE BISTRO" },
              { id: "venue", label: "06 / HACKATHON INFO" },
            ].map((tab) => (
              <button
                key={tab.id}
                onClick={() => scrollToSection(tab.id)}
                className={`px-3 py-1.5 rounded-md transition-all whitespace-nowrap cursor-pointer ${
                  activeTab === tab.id
                    ? "bg-primary/10 text-primary font-bold border-b-2 border-primary"
                    : "hover:text-foreground hover:bg-muted/50"
                }`}
              >
                {tab.label}
              </button>
            ))}
          </nav>
        </div>
      </div>

      <div className="max-w-6xl mx-auto px-4 sm:px-6 lg:px-8 py-16 space-y-24">
        {/* ========================================================= */}
        {/* 4. HOW IT WORKS (3 STEPS)                                  */}
        {/* ========================================================= */}
        <section id="how-it-works" className="scroll-mt-28">
          <div className="text-center max-w-xl mx-auto space-y-2 mb-12">
            <h2 className="text-xs font-mono font-semibold uppercase tracking-wider text-primary">
              How it works
            </h2>
            <p className="text-2xl sm:text-3xl font-semibold tracking-tight text-foreground">
              From natural conversation to confirmed table
            </p>
          </div>

          <motion.div
            variants={containerVariants}
            initial="hidden"
            whileInView="visible"
            viewport={{ once: true, margin: "-50px" }}
            className="grid grid-cols-1 md:grid-cols-3 gap-6"
          >
            {/* Step 1: Ask */}
            <motion.div
              variants={itemVariants}
              className="rounded-xl border border-border/80 bg-card p-6 space-y-3 shadow-xs hover:border-border transition-all"
            >
              <div className="flex size-10 items-center justify-center rounded-lg bg-primary/10 text-primary border border-primary/20">
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
              className="rounded-xl border border-border/80 bg-card p-6 space-y-3 shadow-xs hover:border-border transition-all"
            >
              <div className="flex size-10 items-center justify-center rounded-lg bg-primary/10 text-primary border border-primary/20">
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
              className="rounded-xl border border-border/80 bg-card p-6 space-y-3 shadow-xs hover:border-border transition-all"
            >
              <div className="flex size-10 items-center justify-center rounded-lg bg-primary/10 text-primary border border-primary/20">
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
        {/* 5. CAPABILITY TRACKS & FAST-TRACK BANNER (Screenshot 3)  */}
        {/* ========================================================= */}
        <section id="tracks" className="scroll-mt-28 space-y-8">
          <div className="flex flex-col md:flex-row md:items-end justify-between gap-4">
            <div>
              <div className="text-xs font-mono font-semibold uppercase tracking-wider text-primary">
                02 / CAPABILITIES & TRACKS
              </div>
              <h2 className="text-2xl sm:text-3xl font-semibold tracking-tight text-foreground mt-1">
                Multiple tracks to culinary intelligence
              </h2>
            </div>
            <p className="text-sm text-muted-foreground max-w-md">
              Pick the problem you care about the most: real-time table concurrency, strict allergen safety, or zero-latency pickup checkout.
            </p>
          </div>

          {/* High-Impact Feature Banner (Inspired by Amazon Fast-Track Banner in Screenshot 3) */}
          <div className="relative overflow-hidden rounded-2xl border border-primary/40 bg-gradient-to-r from-sky-950 via-slate-900 to-indigo-950 p-6 sm:p-10 text-white shadow-dropdown">
            {/* Background Pixel & Geometric Accents */}
            <div className="absolute right-4 top-4 opacity-20 pointer-events-none select-none">
              <svg width="220" height="120" viewBox="0 0 220 120" fill="currentColor">
                <rect x="0" y="80" width="40" height="40" />
                <rect x="40" y="60" width="40" height="60" />
                <rect x="80" y="40" width="40" height="80" />
                <rect x="120" y="20" width="40" height="100" />
                <rect x="160" y="0" width="40" height="120" />
              </svg>
            </div>

            <div className="relative max-w-2xl space-y-4">
              <div className="flex flex-wrap items-center gap-2">
                <span className="px-2.5 py-0.5 rounded bg-primary text-primary-foreground font-mono text-[10px] uppercase font-bold tracking-wider">
                  MODEL CONTEXT PROTOCOL
                </span>
                <span className="font-mono text-xs text-sky-200">
                  TOP-TIER LOW LATENCY TOOL EXECUTION
                </span>
              </div>

              <h3 className="text-2xl sm:text-4xl font-bold tracking-tight text-white leading-tight">
                Deterministic voice execution on Rust.
              </h3>

              <p className="text-sm sm:text-base text-slate-300 leading-relaxed">
                TableTalk eliminates hallucination by routing voice requests directly to 24 atomic Rust MCP tools. Instant table locks, real-time allergy cross-contact audits, and integer currency arithmetic.
              </p>

              <div className="flex flex-wrap items-center gap-3 pt-2">
                <div className="px-3 py-1 rounded-md bg-white/10 backdrop-blur-xs border border-white/20 text-xs font-mono text-sky-100">
                  ⚡ 24 MCP TOOLS ACTIVE
                </div>
                <div className="px-3 py-1 rounded-md bg-white/10 backdrop-blur-xs border border-white/20 text-xs font-mono text-sky-100">
                  🔒 ZERO LOGIN REQUIRED
                </div>
                <Link
                  to="/assistant"
                  className="inline-flex items-center text-xs font-semibold text-primary-foreground underline underline-offset-4 hover:text-white transition-colors ml-2"
                >
                  Test MCP Assistant <ArrowRight className="size-3.5 ml-1" />
                </Link>
              </div>
            </div>
          </div>

          {/* 3 Thematic Track Cards (Inspired by Track 01, Track 02, Track 03 in Screenshot 3) */}
          <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
            {/* Track 1: Capacity & Tables */}
            <div className="rounded-xl border border-border/80 bg-card overflow-hidden shadow-xs hover:border-primary/50 transition-all flex flex-col">
              {/* Graphic Header */}
              <div className="h-32 bg-gradient-to-br from-amber-500/20 via-primary/10 to-transparent p-4 flex items-end justify-between border-b border-border/60">
                <div className="flex size-10 items-center justify-center rounded-lg bg-background/80 backdrop-blur-xs text-primary border border-border/60 shadow-2xs">
                  <CalendarCheck className="size-5" />
                </div>
                <span className="font-mono text-[11px] font-bold uppercase tracking-wider text-amber-700 dark:text-amber-400">
                  TRACK 01
                </span>
              </div>
              <div className="p-5 flex-1 flex flex-col justify-between space-y-3">
                <div>
                  <h4 className="text-lg font-semibold text-foreground tracking-tight">
                    Real-Time Capacity
                  </h4>
                  <p className="text-xs sm:text-sm text-muted-foreground leading-relaxed mt-1">
                    Multi-criteria alternative ranking across 14 restaurant tables. If 7:30 PM is booked, TableTalk discovers nearest openings in seconds.
                  </p>
                </div>
                <div className="pt-2 border-t border-border/50 text-[11px] font-mono text-muted-foreground">
                  Lock Window: 15-Minute Grace Period
                </div>
              </div>
            </div>

            {/* Track 2: Allergen Safety */}
            <div className="rounded-xl border border-border/80 bg-card overflow-hidden shadow-xs hover:border-primary/50 transition-all flex flex-col">
              {/* Graphic Header */}
              <div className="h-32 bg-gradient-to-br from-emerald-500/20 via-teal-500/10 to-transparent p-4 flex items-end justify-between border-b border-border/60">
                <div className="flex size-10 items-center justify-center rounded-lg bg-background/80 backdrop-blur-xs text-emerald-600 dark:text-emerald-400 border border-border/60 shadow-2xs">
                  <ShieldCheck className="size-5" />
                </div>
                <span className="font-mono text-[11px] font-bold uppercase tracking-wider text-emerald-700 dark:text-emerald-400">
                  TRACK 02
                </span>
              </div>
              <div className="p-5 flex-1 flex flex-col justify-between space-y-3">
                <div>
                  <h4 className="text-lg font-semibold text-foreground tracking-tight">
                    Allergen Interceptor
                  </h4>
                  <p className="text-xs sm:text-sm text-muted-foreground leading-relaxed mt-1">
                    Kitchen cross-contact disclosures and non-bypassable conflict warnings. Clear risk alerts without false promises.
                  </p>
                </div>
                <div className="pt-2 border-t border-border/50 text-[11px] font-mono text-muted-foreground">
                  Standard: Verbatim Ingredient Audit
                </div>
              </div>
            </div>

            {/* Track 3: Pickup & Payments */}
            <div className="rounded-xl border border-border/80 bg-card overflow-hidden shadow-xs hover:border-primary/50 transition-all flex flex-col">
              {/* Graphic Header */}
              <div className="h-32 bg-gradient-to-br from-indigo-500/20 via-purple-500/10 to-transparent p-4 flex items-end justify-between border-b border-border/60">
                <div className="flex size-10 items-center justify-center rounded-lg bg-background/80 backdrop-blur-xs text-indigo-600 dark:text-indigo-400 border border-border/60 shadow-2xs">
                  <ShoppingBag className="size-5" />
                </div>
                <span className="font-mono text-[11px] font-bold uppercase tracking-wider text-indigo-700 dark:text-indigo-400">
                  TRACK 03
                </span>
              </div>
              <div className="p-5 flex-1 flex flex-col justify-between space-y-3">
                <div>
                  <h4 className="text-lg font-semibold text-foreground tracking-tight">
                    Integer-Cent Cart & Codes
                  </h4>
                  <p className="text-xs sm:text-sm text-muted-foreground leading-relaxed mt-1">
                    Zero floating-point inaccuracies. Dynamic preparation estimation and verifiable curbside order pickup codes.
                  </p>
                </div>
                <div className="pt-2 border-t border-border/50 text-[11px] font-mono text-muted-foreground">
                  Precision: Exact Paisa Currency Math
                </div>
              </div>
            </div>
          </div>
        </section>

        {/* ========================================================= */}
        {/* 6. BENTO FEATURE GRID (6 CELLS - Verified for Tests)      */}
        {/* ========================================================= */}
        <section id="suite" className="scroll-mt-28 py-6 border-t border-border/70">
          <div className="text-center max-w-xl mx-auto space-y-2 mb-12">
            <h2 className="text-xs font-mono font-semibold uppercase tracking-wider text-primary">
              Full Feature Suite
            </h2>
            <p className="text-2xl sm:text-3xl font-semibold tracking-tight text-foreground">
              Production-grade hospitality intelligence
            </p>
          </div>

          <motion.div
            variants={containerVariants}
            initial="hidden"
            whileInView="visible"
            viewport={{ once: true, margin: "-50px" }}
            className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-5"
          >
            {/* Bento 1: Smart Alternatives */}
            <motion.div
              variants={itemVariants}
              className="rounded-xl border border-border/80 bg-card p-6 space-y-3 shadow-xs hover:border-border transition-colors"
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
              className="rounded-xl border border-border/80 bg-card p-6 space-y-3 shadow-xs hover:border-border transition-colors"
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
              className="rounded-xl border border-border/80 bg-card p-6 space-y-3 shadow-xs hover:border-border transition-colors"
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
              className="rounded-xl border border-border/80 bg-card p-6 space-y-3 shadow-xs hover:border-border transition-colors"
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
              className="rounded-xl border border-border/80 bg-card p-6 space-y-3 shadow-xs hover:border-border transition-colors"
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
              className="rounded-xl border border-border/80 bg-card p-6 space-y-3 shadow-xs hover:border-border transition-colors"
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
        {/* 7. THE BISTRO ATMOSPHERE (Inspired by Screenshot 2 Gallery) */}
        {/* ========================================================= */}
        <section id="atmosphere" className="scroll-mt-28 space-y-8">
          <div className="flex flex-col md:flex-row md:items-end justify-between gap-4">
            <div>
              <div className="text-xs font-mono font-semibold uppercase tracking-wider text-primary">
                35 / HANDCRAFTED DISHES & MOMENTS
              </div>
              <h2 className="text-2xl sm:text-3xl font-semibold tracking-tight text-foreground mt-1">
                The day in the bistro
              </h2>
            </div>
            <p className="text-sm text-muted-foreground max-w-md">
              From the 800°F clay tandoor to tranquil heated patio banquettes: experience authentic culinary craft.
            </p>
          </div>

          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-5">
            {/* Moment 1 */}
            <div className="rounded-xl border border-border/80 bg-card p-5 space-y-3 shadow-xs hover:border-primary/40 transition-all">
              <div className="h-28 rounded-lg bg-gradient-to-br from-amber-500/20 via-primary/10 to-transparent flex items-center justify-center text-primary">
                <Flame className="size-8" />
              </div>
              <div className="text-[10px] font-mono text-muted-foreground uppercase">Hearth 01</div>
              <h4 className="text-sm font-semibold text-foreground">800°F Clay Tandoors</h4>
              <p className="text-xs text-muted-foreground leading-relaxed">
                Slow-roasted Bhatti chicken and smoked seekh kebabs charred with live wood embers.
              </p>
            </div>

            {/* Moment 2 */}
            <div className="rounded-xl border border-border/80 bg-card p-5 space-y-3 shadow-xs hover:border-primary/40 transition-all">
              <div className="h-28 rounded-lg bg-gradient-to-br from-emerald-500/20 via-teal-500/10 to-transparent flex items-center justify-center text-emerald-600 dark:text-emerald-400">
                <Coffee className="size-8" />
              </div>
              <div className="text-[10px] font-mono text-muted-foreground uppercase">Atelier 02</div>
              <h4 className="text-sm font-semibold text-foreground">The Spice Sanctuary</h4>
              <p className="text-xs text-muted-foreground leading-relaxed">
                Hand-pounded tellicherry peppercorns, green cardamom, and Kashmiri saffron strands.
              </p>
            </div>

            {/* Moment 3 */}
            <div className="rounded-xl border border-border/80 bg-card p-5 space-y-3 shadow-xs hover:border-primary/40 transition-all">
              <div className="h-28 rounded-lg bg-gradient-to-br from-indigo-500/20 via-sky-500/10 to-transparent flex items-center justify-center text-sky-600 dark:text-sky-400">
                <Radio className="size-8" />
              </div>
              <div className="text-[10px] font-mono text-muted-foreground uppercase">Tech 03</div>
              <h4 className="text-sm font-semibold text-foreground">Instant Code Check-in</h4>
              <p className="text-xs text-muted-foreground leading-relaxed">
                6-character alphanumeric booking code allows direct walk-in seating with zero wait.
              </p>
            </div>

            {/* Moment 4 */}
            <div className="rounded-xl border border-border/80 bg-card p-5 space-y-3 shadow-xs hover:border-primary/40 transition-all">
              <div className="h-28 rounded-lg bg-gradient-to-br from-purple-500/20 via-primary/10 to-transparent flex items-center justify-center text-purple-600 dark:text-purple-400">
                <UtensilsCrossed className="size-8" />
              </div>
              <div className="text-[10px] font-mono text-muted-foreground uppercase">Menu 04</div>
              <h4 className="text-sm font-semibold text-foreground">35 Curated Dishes</h4>
              <p className="text-xs text-muted-foreground leading-relaxed">
                Every single starter, grill, curry, biryani, and dessert has authentic culinary art and nutrition notes.
              </p>
            </div>
          </div>
        </section>

        {/* ========================================================= */}
        {/* 8. IN PERSON & WHO YOU WILL MEET (Inspired by Screenshot 4)*/}
        {/* ========================================================= */}
        <section id="venue" className="scroll-mt-28 space-y-8">
          <div className="flex flex-col md:flex-row md:items-end justify-between gap-4">
            <div>
              <div className="text-xs font-mono font-semibold uppercase tracking-wider text-primary">
                08 / IN PERSON & HOSTING
              </div>
              <h2 className="text-2xl sm:text-3xl font-semibold tracking-tight text-foreground mt-1">
                Spend the evening with us at The Saffron Nebula
              </h2>
            </div>
            <p className="text-sm text-muted-foreground max-w-md">
              Crafted for the voice developer challenge. Explore the people and engines behind the experience.
            </p>
          </div>

          {/* Venue & Host Box */}
          <div className="rounded-2xl border border-border/80 bg-card p-6 space-y-6 shadow-xs">
            <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4 pb-4 border-b border-border/60">
              <div className="space-y-1">
                <div className="flex items-center gap-2 text-xs font-mono text-muted-foreground">
                  <MapPin className="size-3.5 text-primary" />
                  <span>CONNAUGHT PLACE, NEW DELHI & ONLINE WORLDWIDE</span>
                </div>
                <h3 className="text-lg font-semibold text-foreground">
                  The Saffron Nebula Bistro & DTU Delhi
                </h3>
              </div>
              <div className="flex items-center gap-2">
                <span className="px-2.5 py-1 rounded-md bg-muted text-xs font-mono font-medium text-foreground">
                  HOSTED WITH MCP
                </span>
                <span className="px-2.5 py-1 rounded-md bg-primary/10 text-primary text-xs font-mono font-bold">
                  ALEXA+ TRACK
                </span>
              </div>
            </div>

            {/* Who You Will Meet Grid */}
            <div className="space-y-3">
              <div className="text-xs font-mono uppercase tracking-wider text-muted-foreground">
                WHO YOU WILL MEET / INTELLIGENCE & CULINARY TEAM
              </div>

              <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
                {/* Person 1 */}
                <div className="rounded-xl border border-border/70 bg-muted/20 p-4 space-y-2">
                  <div className="flex items-center gap-3">
                    <div className="flex size-10 items-center justify-center rounded-full bg-primary/10 text-primary font-bold text-sm">
                      AS
                    </div>
                    <div>
                      <div className="text-sm font-semibold text-foreground flex items-center gap-1">
                        Chef Aarav Sen
                        <ExternalLink className="size-3 text-muted-foreground" />
                      </div>
                      <div className="text-[11px] text-muted-foreground">Executive Chef & Master of Dum</div>
                    </div>
                  </div>
                  <p className="text-xs text-muted-foreground leading-relaxed pt-1">
                    Guardian of traditional hand-pounded spices, slow-ember curries, and authentic Awadhi biryanis.
                  </p>
                </div>

                {/* Person 2 */}
                <div className="rounded-xl border border-border/70 bg-muted/20 p-4 space-y-2">
                  <div className="flex items-center gap-3">
                    <div className="flex size-10 items-center justify-center rounded-full bg-emerald-500/10 text-emerald-600 dark:text-emerald-400 font-bold text-sm">
                      TT
                    </div>
                    <div>
                      <div className="text-sm font-semibold text-foreground flex items-center gap-1">
                        TableTalk MCP Agent
                        <ExternalLink className="size-3 text-muted-foreground" />
                      </div>
                      <div className="text-[11px] text-muted-foreground">Voice Reasoning & Table Arbiter</div>
                    </div>
                  </div>
                  <p className="text-xs text-muted-foreground leading-relaxed pt-1">
                    Powered by 24 atomic Rust tools. Executes conflict-free bookings and allergen validations in under 30ms.
                  </p>
                </div>

                {/* Person 3 */}
                <div className="rounded-xl border border-border/70 bg-muted/20 p-4 space-y-2">
                  <div className="flex items-center gap-3">
                    <div className="flex size-10 items-center justify-center rounded-full bg-indigo-500/10 text-indigo-600 dark:text-indigo-400 font-bold text-sm">
                      MK
                    </div>
                    <div>
                      <div className="text-sm font-semibold text-foreground flex items-center gap-1">
                        Meera Kapoor
                        <ExternalLink className="size-3 text-muted-foreground" />
                      </div>
                      <div className="text-[11px] text-muted-foreground">Hospitality Concierge & Sommelier</div>
                    </div>
                  </div>
                  <p className="text-xs text-muted-foreground leading-relaxed pt-1">
                    Curator of single-estate Darjeeling teas, kokum coolers, and personalized dining experiences.
                  </p>
                </div>
              </div>
            </div>

            {/* Bottom 4 Feature Columns (Screenshot 4 Style) */}
            <div className="grid grid-cols-2 md:grid-cols-4 gap-4 pt-4 border-t border-border/60 text-xs">
              <div>
                <h5 className="font-semibold text-foreground">Interactive Voice</h5>
                <p className="text-muted-foreground text-[11px] mt-0.5">
                  Natural multi-turn conversation with SSE streaming audio.
                </p>
              </div>
              <div>
                <h5 className="font-semibold text-foreground">Allergen Audits</h5>
                <p className="text-muted-foreground text-[11px] mt-0.5">
                  Strict cross-contact warnings with kitchen verification.
                </p>
              </div>
              <div>
                <h5 className="font-semibold text-foreground">Alexa+ Challenge</h5>
                <p className="text-muted-foreground text-[11px] mt-0.5">
                  Demonstrating next-generation MCP agent tooling.
                </p>
              </div>
              <div>
                <h5 className="font-semibold text-foreground">Zero-Login Freedom</h5>
                <p className="text-muted-foreground text-[11px] mt-0.5">
                  Direct tokenized sessions with full privacy deletion.
                </p>
              </div>
            </div>
          </div>
        </section>

        {/* ========================================================= */}
        {/* 9. TRUST STRIP                                            */}
        {/* ========================================================= */}
        <section className="py-2">
          <div className="flex flex-col sm:flex-row items-center justify-between gap-4 rounded-xl border border-border/80 bg-muted/30 px-6 py-4 text-xs text-muted-foreground">
            <div className="flex items-center gap-2.5 text-foreground font-medium">
              <ShieldCheck className="size-4 text-primary shrink-0" />
              <span>Demo project. No real payments. Your data can be deleted any time.</span>
            </div>
            <div className="flex items-center gap-4 shrink-0">
              <Link to="/privacy" className="hover:text-foreground underline underline-offset-4 transition-colors">
                Privacy Policy
              </Link>
              <span className="text-border">·</span>
              <span>Zero telemetry</span>
            </div>
          </div>
        </section>

        {/* ========================================================= */}
        {/* 10. GRAND CLOSING HERO (Inspired by Screenshot 5)         */}
        {/* ========================================================= */}
        <section className="py-8 text-center">
          <div className="relative overflow-hidden rounded-3xl border border-border/90 bg-gradient-to-b from-card via-muted/30 to-background p-8 sm:p-14 space-y-6 max-w-4xl mx-auto shadow-dropdown">
            <div className="inline-flex items-center gap-2 px-3 py-1 rounded-full bg-primary/10 text-primary text-xs font-mono font-semibold uppercase">
              EVENT 02 OF 05
            </div>

            <h2 className="text-3xl sm:text-5xl font-semibold tracking-tight text-foreground leading-tight">
              Build what hospitality needs.
            </h2>

            <p className="text-sm sm:text-base text-muted-foreground max-w-xl mx-auto leading-relaxed">
              Four days, three tracks, and a problem worth fixing: seamless, accessible dining intelligence with zero apps to install and zero passwords to remember.
            </p>

            <div className="pt-2 flex flex-wrap items-center justify-center gap-4">
              <Button asChild size="lg" className="h-11 px-8 text-sm font-medium shadow-sm">
                <Link to="/assistant">
                  <Mic className="size-4 mr-2" />
                  Talk to the assistant
                  <ArrowRight className="size-4 ml-2" />
                </Link>
              </Button>
            </div>

            <p className="text-[11px] font-mono uppercase tracking-wider text-muted-foreground/80 pt-2">
              DTU IN DELHI HOSTS THE BUILD DAY. ONLINE IS OPEN TO ALL OF INDIA.
            </p>
          </div>
        </section>

        {/* ========================================================= */}
        {/* 11. EDITORIAL FOOTER                                      */}
        {/* ========================================================= */}
        <footer className="pt-8 border-t border-border/60 text-xs text-muted-foreground">
          <div className="flex flex-col sm:flex-row items-center justify-between gap-4">
            <div className="flex items-center gap-1.5 font-medium text-foreground">
              <span>TableTalk</span>
              <span className="size-1 rounded-full bg-primary" />
              <span className="text-muted-foreground text-[11px] font-normal ml-2">
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
