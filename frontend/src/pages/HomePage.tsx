import { useState, useEffect } from "react"
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
} from "lucide-react"
import { Button } from "@/components/ui/button"
import { StatusBadge } from "@/components/primitives/StatusBadge"
import { CodeBlock } from "@/components/primitives/CodeBlock"

export function HomePage() {
  const shouldReduceMotion = useReducedMotion()

  // Typing animation simulation for the conversation preview
  const [step, setStep] = useState(0)

  useEffect(() => {
    if (shouldReduceMotion) {
      setStep(3)
      return
    }

    const t1 = setTimeout(() => setStep(1), 400) // user message
    const t2 = setTimeout(() => setStep(2), 1100) // assistant response
    const t3 = setTimeout(() => setStep(3), 1700) // card reveal

    return () => {
      clearTimeout(t1)
      clearTimeout(t2)
      clearTimeout(t3)
    }
  }, [shouldReduceMotion])

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

  return (
    <div className="relative overflow-hidden">
      {/* Background radial gradient glow (pure CSS, subtle warmth) */}
      <div
        className="pointer-events-none absolute -top-40 left-1/2 -z-10 h-[520px] w-[900px] -translate-x-1/2 rounded-full bg-gradient-to-b from-primary/10 via-primary/5 to-transparent blur-3xl opacity-75 dark:opacity-40"
        aria-hidden="true"
      />

      <div className="max-w-6xl mx-auto px-4 sm:px-6 lg:px-8 pt-6 sm:pt-12 pb-20">
        {/* ========================================================= */}
        {/* 1. HERO SECTION                                           */}
        {/* ========================================================= */}
        <section className="grid grid-cols-1 lg:grid-cols-12 gap-12 lg:gap-8 items-center pt-2 sm:pt-6 pb-16 lg:pb-24">
          {/* Left Column: Headline & Value Proposition */}
          <div className="lg:col-span-6 space-y-6">
            <div className="inline-flex items-center gap-2 rounded-full border border-border/80 bg-muted/60 px-3 py-1 text-xs font-medium text-foreground/90 backdrop-blur-xs">
              <span className="flex size-2 rounded-full bg-primary animate-pulse" />
              <span>Voice-first restaurant intelligence</span>
            </div>

            <h1 className="text-4xl sm:text-5xl lg:text-6xl font-semibold tracking-tight text-foreground leading-[1.08]">
              Book a table by just asking.
            </h1>

            <p className="text-base sm:text-lg text-muted-foreground leading-relaxed max-w-xl">
              An intelligent dining assistant that checks live table capacity, negotiates alternatives, and manages pickup orders naturally — powered by the Model Context Protocol.
            </p>

            <div className="flex flex-wrap items-center gap-3 pt-2">
              <Button asChild size="lg" className="h-11 px-5 text-sm font-medium shadow-sm">
                <Link to="/assistant">
                  <Mic className="size-4 mr-2" />
                  Talk to the assistant
                  <ArrowRight className="size-4 ml-2" />
                </Link>
              </Button>

              <Button asChild variant="outline" size="lg" className="h-11 px-5 text-sm font-medium">
                <Link to="/menu">
                  <UtensilsCrossed className="size-4 mr-2" />
                  View menu
                </Link>
              </Button>
            </div>

            <div className="flex items-center gap-6 pt-4 text-xs text-muted-foreground border-t border-border/60">
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
          </div>

          {/* Right Column: Animated Conversation & Booking Card Mock */}
          <div className="lg:col-span-6">
            <div className="relative rounded-2xl border border-border/80 bg-card p-5 sm:p-6 shadow-dropdown transition-all">
              <div className="flex items-center justify-between pb-4 mb-4 border-b border-border/60 text-xs">
                <div className="flex items-center gap-2">
                  <span className="size-2.5 rounded-full bg-emerald-500" />
                  <span className="font-semibold text-foreground">The Saffron Nebula Bistro</span>
                </div>
                <span className="text-[11px] text-muted-foreground font-mono">Live Session</span>
              </div>

              {/* Chat Thread */}
              <div className="space-y-4">
                {/* User Message */}
                {step >= 1 && (
                  <motion.div
                    initial={shouldReduceMotion ? { opacity: 1 } : { opacity: 0, y: 10 }}
                    animate={{ opacity: 1, y: 0 }}
                    transition={{ duration: 0.2 }}
                    className="flex justify-end"
                  >
                    <div className="max-w-[85%] rounded-2xl rounded-tr-xs bg-primary px-4 py-2.5 text-xs sm:text-sm text-primary-foreground leading-relaxed shadow-xs">
                      Can we get a table for four this Friday around 7:30 PM? We'd love the patio.
                    </div>
                  </motion.div>
                )}

                {/* Assistant Message */}
                {step >= 2 && (
                  <motion.div
                    initial={shouldReduceMotion ? { opacity: 1 } : { opacity: 0, y: 10 }}
                    animate={{ opacity: 1, y: 0 }}
                    transition={{ duration: 0.2 }}
                    className="flex justify-start"
                  >
                    <div className="max-w-[90%] rounded-2xl rounded-tl-xs border border-border/80 bg-muted/40 px-4 py-2.5 text-xs sm:text-sm text-foreground leading-relaxed">
                      I found an open table for four on the patio this Friday, October 16 at 7:30 PM. I've confirmed your reservation.
                    </div>
                  </motion.div>
                )}

                {/* Booking Confirmation Card Mock */}
                {step >= 3 && (
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
              </div>
            </div>
          </div>
        </section>

        {/* ========================================================= */}
        {/* 2. HOW IT WORKS (3 STEPS)                                  */}
        {/* ========================================================= */}
        <section className="py-16 border-t border-border/70">
          <div className="text-center max-w-xl mx-auto space-y-2 mb-12">
            <h2 className="text-xs font-semibold uppercase tracking-wider text-primary">
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
              className="rounded-xl border border-border/80 bg-card p-6 space-y-3 shadow-xs"
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
              className="rounded-xl border border-border/80 bg-card p-6 space-y-3 shadow-xs"
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
              className="rounded-xl border border-border/80 bg-card p-6 space-y-3 shadow-xs"
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
        {/* 3. BENTO FEATURE GRID (6 CELLS)                           */}
        {/* ========================================================= */}
        <section className="py-16 border-t border-border/70">
          <div className="text-center max-w-xl mx-auto space-y-2 mb-12">
            <h2 className="text-xs font-semibold uppercase tracking-wider text-primary">
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
        {/* 4. TRUST STRIP                                            */}
        {/* ========================================================= */}
        <section className="py-6">
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
        {/* 5. FINAL CTA BAND                                         */}
        {/* ========================================================= */}
        <section className="py-16 text-center">
          <div className="rounded-2xl border border-border/90 bg-gradient-to-b from-card to-muted/20 p-8 sm:p-12 space-y-5 max-w-3xl mx-auto shadow-xs">
            <h2 className="text-2xl sm:text-4xl font-semibold tracking-tight text-foreground">
              Experience the assistant now
            </h2>
            <p className="text-sm sm:text-base text-muted-foreground max-w-lg mx-auto leading-relaxed">
              Launch the interactive voice interface to explore availability, negotiate reservations, or simulate pickup orders.
            </p>
            <div className="pt-2">
              <Button asChild size="lg" className="h-11 px-6 text-sm font-medium shadow-sm">
                <Link to="/assistant">
                  <Mic className="size-4 mr-2" />
                  Talk to the assistant
                  <ArrowRight className="size-4 ml-2" />
                </Link>
              </Button>
            </div>
          </div>
        </section>

        {/* ========================================================= */}
        {/* 6. MINIMAL FOOTER                                         */}
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
