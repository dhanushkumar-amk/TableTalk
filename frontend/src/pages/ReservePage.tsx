import { useState } from "react"
import { Link, useNavigate, useSearchParams } from "react-router-dom"
import { useForm } from "react-hook-form"
import { z } from "zod"
import { motion, useReducedMotion } from "framer-motion"
import {
  Clock,
  CheckCircle2,
  AlertCircle,
  CreditCard,
  Download,
  ArrowRight,
  ArrowLeft,
} from "lucide-react"
import { Button } from "@/components/ui/button"
import { Money } from "@/components/primitives/Money"
import { CodeBlock } from "@/components/primitives/CodeBlock"
import { CountdownChip } from "@/components/primitives/CountdownChip"
import { SimulatedBanner } from "@/components/primitives/SimulatedBanner"
import { downloadCalendarEvent } from "@/lib/calendar"

const customZodResolver = (schema: z.ZodSchema<any>) => async (data: any) => {
  const result = schema.safeParse(data)
  if (result.success) {
    return { values: result.data, errors: {} }
  }
  const errors: Record<string, { type: string; message: string }> = {}
  result.error.issues.forEach((issue) => {
    const key = issue.path[0] as string
    if (!errors[key]) {
      errors[key] = { type: issue.code, message: issue.message }
    }
  })
  return { values: {}, errors }
}

// Zod Schema for Guest Details
const guestDetailsSchema = z.object({
  fullName: z
    .string()
    .min(2, "Full name must be at least 2 characters")
    .max(50, "Full name must be under 50 characters"),
  phone: z
    .string()
    .optional()
    .refine((val) => !val || /^[+]?[0-9\s-]{8,15}$/.test(val), {
      message: "Please enter a valid phone number",
    }),
  email: z
    .string()
    .optional()
    .refine((val) => !val || /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(val), {
      message: "Please enter a valid email address",
    }),
  specialRequests: z
    .string()
    .max(200, "Special requests must be under 200 characters")
    .optional(),
  saveProfileConsent: z.boolean().default(false),
})

type GuestDetailsForm = z.infer<typeof guestDetailsSchema>

const LUNCH_TIMES = ["12:00", "12:30", "13:00", "13:30", "14:00", "14:30"]
const DINNER_TIMES = ["19:00", "19:30", "20:00", "20:30", "21:00", "21:30", "22:00"]
const AREAS = [
  { id: "any", label: "Any Area" },
  { id: "indoor", label: "Indoor Dining" },
  { id: "patio", label: "Garden Patio" },
  { id: "bar", label: "Chef's Bar" },
]

export function ReservePage() {
  const [searchParams] = useSearchParams()
  const navigate = useNavigate()
  const shouldReduceMotion = useReducedMotion()

  // Booking Step: 1 = Party/Time, 2 = Availability, 3 = Details, 4 = Deposit, 5 = Confirmed
  const [step, setStep] = useState<1 | 2 | 3 | 4 | 5>(1)

  // Step 1 State
  const tomorrow = new Date()
  tomorrow.setDate(tomorrow.getDate() + 1)
  const defaultDateStr = tomorrow.toISOString().split("T")[0]

  const [partySize, setPartySize] = useState<number>(() => {
    const p = Number(searchParams.get("party"))
    return p >= 1 && p <= 12 ? p : 2
  })
  const [selectedDate, setSelectedDate] = useState<string>(
    searchParams.get("date") || defaultDateStr
  )
  const [selectedTime, setSelectedTime] = useState<string>(
    searchParams.get("time") || "19:30"
  )
  const [selectedArea, setSelectedArea] = useState<string>(
    searchParams.get("area") || "indoor"
  )

  // Step 2 Availability state
  const [isCheckingAvailability, setIsCheckingAvailability] = useState(false)
  const [hasConflict, setHasConflict] = useState(false)
  const [selectedAlternative, setSelectedAlternative] = useState<{
    time: string
    area: string
    reason: string
  } | null>(null)

  // Step 4 Deposit state
  const isDepositRequired = partySize >= 8
  const [depositPaid, setDepositPaid] = useState(false)
  const depositDeadline = new Date(Date.now() + 15 * 60 * 1000) // 15 mins hold

  // Step 5 Confirmation state
  const [confirmationCode, setConfirmationCode] = useState("TT-849201")

  // Form setup
  const {
    register,
    handleSubmit,
    watch,
    formState: { errors },
  } = useForm<GuestDetailsForm>({
    resolver: customZodResolver(guestDetailsSchema),
    defaultValues: {
      fullName: "",
      phone: "",
      email: "",
      specialRequests: "",
      saveProfileConsent: false,
    },
  })

  const specialRequestsText = watch("specialRequests") || ""
  const guestName = watch("fullName")

  // Availability Check Handler
  const handleCheckAvailability = () => {
    setIsCheckingAvailability(true)
    setTimeout(() => {
      setIsCheckingAvailability(false)
      // If user selected 20:00 on Friday, simulate realistic peak-time conflict
      if (selectedTime === "20:00") {
        setHasConflict(true)
      } else {
        setHasConflict(false)
      }
      setStep(2)
    }, 40)
  }

  // Submit Details
  const onDetailsSubmit = (_data: GuestDetailsForm) => {
    if (isDepositRequired && !depositPaid) {
      setStep(4)
    } else {
      const code = `TT-${Math.floor(100000 + Math.random() * 900000)}`
      setConfirmationCode(code)
      setStep(5)
    }
  }

  // Handle Calendar Download
  const handleDownloadCalendar = () => {
    const timeParts = (selectedAlternative?.time || selectedTime).split(":")
    const dateObj = new Date(selectedDate)
    dateObj.setHours(Number(timeParts[0]), Number(timeParts[1]), 0)

    downloadCalendarEvent({
      title: `TableTalk Dining Reservation (${partySize} guests)`,
      description: `Reservation at The Saffron Nebula Bistro for ${guestName || "Guest"}. Seating: ${selectedAlternative?.area || selectedArea}.`,
      location: "The Saffron Nebula Bistro, Bengaluru",
      startDateIso: dateObj.toISOString(),
      durationMinutes: 90,
      confirmationCode,
    })
  }

  return (
    <div className="relative min-h-[calc(100vh-3.5rem)] bg-background pb-24">
      {/* ========================================================= */}
      {/* PROGRESS STEPPER HEADER                                   */}
      {/* ========================================================= */}
      <header className="border-b border-border/80 bg-card/40 backdrop-blur-xs px-4 py-4">
        <div className="mx-auto max-w-5xl">
          <div className="flex items-center justify-between text-xs text-muted-foreground pb-2">
            <span className="font-medium text-foreground">
              Step {step} of {isDepositRequired ? 5 : 4}
            </span>
            <span>
              {step === 1 && "Party & Time"}
              {step === 2 && "Availability Result"}
              {step === 3 && "Guest Details"}
              {step === 4 && "Deposit Hold"}
              {step === 5 && "Confirmed"}
            </span>
          </div>

          {/* Stepper Bar */}
          <div className="grid grid-cols-4 gap-2">
            {[1, 2, 3, isDepositRequired ? 4 : 5].map((_, idx) => {
              const isActive = step >= (idx + 1)
              return (
                <div
                  key={idx}
                  className={`h-1.5 rounded-full transition-all duration-300 ${
                    isActive ? "bg-primary" : "bg-muted"
                  }`}
                />
              )
            })}
          </div>
        </div>
      </header>

      {/* ========================================================= */}
      {/* MAIN LAYOUT: CONTENT + STICKY SIDEBAR                     */}
      {/* ========================================================= */}
      <div className="mx-auto max-w-5xl px-4 pt-8">
        <div className="grid grid-cols-1 lg:grid-cols-3 gap-8">
          {/* Main Flow (2 cols on desktop) */}
          <div className="lg:col-span-2 space-y-6">
            {/* STEP 1: PARTY & TIME */}
            {step === 1 && (
              <motion.div
                initial={shouldReduceMotion ? { opacity: 1 } : { opacity: 0, y: 12 }}
                animate={{ opacity: 1, y: 0 }}
                className="rounded-2xl border border-border/80 bg-card p-6 shadow-xs space-y-6"
              >
                <div>
                  <h1 className="text-xl sm:text-2xl font-semibold tracking-tight text-foreground">
                    Reserve a Table
                  </h1>
                  <p className="text-xs sm:text-sm text-muted-foreground leading-relaxed pt-1">
                    Select your party size, date, preferred dining area, and meal service time.
                  </p>
                </div>

                {/* 1. Party Size Stepper */}
                <div className="space-y-2">
                  <label className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
                    Party Size
                  </label>
                  <div className="flex items-center gap-4">
                    <div className="flex items-center rounded-xl border border-border/80 bg-muted/30 p-1">
                      <button
                        type="button"
                        disabled={partySize <= 1}
                        onClick={() => setPartySize((p) => Math.max(1, p - 1))}
                        className="flex size-10 items-center justify-center rounded-lg text-lg font-bold text-foreground hover:bg-card disabled:opacity-30 disabled:pointer-events-none transition-colors"
                        aria-label="Decrease party size"
                      >
                        -
                      </button>
                      <span className="w-14 text-center text-base font-semibold text-foreground">
                        {partySize} {partySize === 1 ? "guest" : "guests"}
                      </span>
                      <button
                        type="button"
                        disabled={partySize >= 12}
                        onClick={() => setPartySize((p) => Math.min(12, p + 1))}
                        className="flex size-10 items-center justify-center rounded-lg text-lg font-bold text-foreground hover:bg-card disabled:opacity-30 disabled:pointer-events-none transition-colors"
                        aria-label="Increase party size"
                      >
                        +
                      </button>
                    </div>

                    {partySize >= 8 && (
                      <span className="text-xs text-amber-600 dark:text-amber-400 font-medium flex items-center gap-1">
                        <CreditCard className="size-3.5" />
                        Requires ₹2,000 deposit hold
                      </span>
                    )}
                  </div>
                </div>

                {/* 2. Date Picker */}
                <div className="space-y-2">
                  <label htmlFor="date-input" className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
                    Date
                  </label>
                  <div className="relative max-w-xs">
                    <input
                      id="date-input"
                      type="date"
                      value={selectedDate}
                      min={new Date().toISOString().split("T")[0]}
                      onChange={(e) => setSelectedDate(e.target.value)}
                      className="h-11 w-full rounded-xl border border-border/80 bg-muted/20 px-3.5 text-sm text-foreground focus:border-ring focus:outline-none focus:ring-2 focus:ring-ring/20"
                    />
                  </div>
                </div>

                {/* 3. Seating Area Segmented Control */}
                <div className="space-y-2">
                  <label className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
                    Dining Area
                  </label>
                  <div className="grid grid-cols-2 sm:grid-cols-4 gap-2">
                    {AREAS.map((area) => {
                      const isSelected = selectedArea === area.id
                      return (
                        <button
                          key={area.id}
                          type="button"
                          onClick={() => setSelectedArea(area.id)}
                          className={`min-h-[44px] rounded-xl border p-2.5 text-xs font-medium transition-all ${
                            isSelected
                              ? "border-primary bg-primary/10 text-primary font-semibold shadow-2xs"
                              : "border-border/80 bg-muted/20 text-muted-foreground hover:text-foreground hover:border-border"
                          }`}
                        >
                          {area.label}
                        </button>
                      )
                    })}
                  </div>
                </div>

                {/* 4. Time Chips Grouped by Lunch / Dinner */}
                <div className="space-y-4">
                  <div className="space-y-2">
                    <div className="flex items-center gap-2 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
                      <Clock className="size-3.5 text-primary" />
                      <span>Lunch Service (12:00 PM – 3:30 PM)</span>
                    </div>
                    <div className="flex flex-wrap gap-2">
                      {LUNCH_TIMES.map((time) => {
                        const isSelected = selectedTime === time
                        return (
                          <button
                            key={time}
                            type="button"
                            onClick={() => setSelectedTime(time)}
                            className={`min-h-[44px] px-3.5 py-2 rounded-xl text-xs font-medium transition-all ${
                              isSelected
                                ? "bg-primary text-primary-foreground font-semibold shadow-2xs"
                                : "border border-border/80 bg-muted/20 text-muted-foreground hover:text-foreground hover:border-border"
                            }`}
                          >
                            {time}
                          </button>
                        )
                      })}
                    </div>
                  </div>

                  <div className="space-y-2">
                    <div className="flex items-center gap-2 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
                      <Clock className="size-3.5 text-primary" />
                      <span>Dinner Service (7:00 PM – 11:30 PM)</span>
                    </div>
                    <div className="flex flex-wrap gap-2">
                      {DINNER_TIMES.map((time) => {
                        const isSelected = selectedTime === time
                        return (
                          <button
                            key={time}
                            type="button"
                            onClick={() => setSelectedTime(time)}
                            className={`min-h-[44px] px-3.5 py-2 rounded-xl text-xs font-medium transition-all ${
                              isSelected
                                ? "bg-primary text-primary-foreground font-semibold shadow-2xs"
                                : "border border-border/80 bg-muted/20 text-muted-foreground hover:text-foreground hover:border-border"
                            }`}
                          >
                            {time}
                          </button>
                        )
                      })}
                    </div>
                  </div>
                </div>

                {/* Submit Action */}
                <div className="pt-2">
                  <Button
                    size="lg"
                    disabled={isCheckingAvailability}
                    onClick={handleCheckAvailability}
                    className="h-11 w-full text-sm font-medium shadow-xs"
                  >
                    {isCheckingAvailability ? "Checking capacity..." : "Check Table Availability"}
                    <ArrowRight className="size-4 ml-2" />
                  </Button>
                </div>
              </motion.div>
            )}

            {/* STEP 2: AVAILABILITY RESULT */}
            {step === 2 && (
              <motion.div
                initial={shouldReduceMotion ? { opacity: 1 } : { opacity: 0, y: 12 }}
                animate={{ opacity: 1, y: 0 }}
                className="space-y-6"
              >
                {!hasConflict ? (
                  // Available slot card
                  <div className="rounded-2xl border border-border/80 bg-card p-6 shadow-xs space-y-5">
                    <div className="flex items-center gap-2 text-emerald-600 dark:text-emerald-400">
                      <CheckCircle2 className="size-5" />
                      <h2 className="text-lg font-semibold tracking-tight text-foreground">
                        Table Available!
                      </h2>
                    </div>

                    <div className="rounded-xl border border-border/80 bg-muted/20 p-4 space-y-3">
                      <div className="flex items-center justify-between">
                        <span className="text-xs text-muted-foreground">Confirmed Table Slot</span>
                        <span className="font-mono text-xs font-semibold text-primary">Table T01</span>
                      </div>
                      <div className="grid grid-cols-2 gap-3 text-xs">
                        <div>
                          <span className="text-muted-foreground block text-[11px]">Time Window</span>
                          <strong className="text-foreground">{selectedTime} – {selectedTime === "19:30" ? "20:45" : "90 mins"}</strong>
                        </div>
                        <div>
                          <span className="text-muted-foreground block text-[11px]">Area & Capacity</span>
                          <strong className="text-foreground capitalize">{selectedArea} · {partySize} Guests</strong>
                        </div>
                      </div>
                    </div>

                    {/* Snapshot note verbatim */}
                    <div className="rounded-lg border border-border/60 bg-muted/30 p-3 text-xs text-muted-foreground flex items-center gap-2">
                      <AlertCircle className="size-4 text-primary shrink-0" />
                      <span>Availability is a snapshot, not a reservation. Complete details below to secure this table.</span>
                    </div>

                    <div className="flex items-center gap-3 pt-2">
                      <Button
                        variant="outline"
                        onClick={() => setStep(1)}
                        className="h-11 text-xs"
                      >
                        <ArrowLeft className="size-3.5 mr-1.5" />
                        Modify Time
                      </Button>
                      <Button
                        onClick={() => setStep(3)}
                        className="h-11 flex-1 text-xs font-medium shadow-xs"
                      >
                        Continue to Guest Details
                        <ArrowRight className="size-3.5 ml-1.5" />
                      </Button>
                    </div>
                  </div>
                ) : (
                  // Conflict: Ranked alternatives cards
                  <div className="rounded-2xl border border-border/80 bg-card p-6 shadow-xs space-y-5">
                    <div className="flex items-center gap-2 text-amber-600 dark:text-amber-400">
                      <AlertCircle className="size-5" />
                      <h2 className="text-lg font-semibold tracking-tight text-foreground">
                        Requested Time is Fully Booked
                      </h2>
                    </div>

                    <p className="text-xs sm:text-sm text-muted-foreground leading-relaxed">
                      We couldn't accommodate {partySize} guests at exactly {selectedTime} in the {selectedArea} area. Here are intelligent ranked alternatives:
                    </p>

                    {/* Ranked Alternatives */}
                    <div className="space-y-3">
                      {[
                        { time: "19:30", area: "Indoor", reason: "Closest to your time (-30m)" },
                        { time: "21:00", area: "Indoor", reason: "Same area (+60m)" },
                        { time: "20:00", area: "Patio", reason: "Exact time in alternative area" },
                      ].map((alt, i) => (
                        <div
                          key={i}
                          onClick={() => {
                            setSelectedAlternative(alt)
                            setSelectedTime(alt.time)
                            setSelectedArea(alt.area.toLowerCase())
                          }}
                          className={`cursor-pointer rounded-xl border p-4 transition-all flex items-center justify-between ${
                            selectedAlternative?.time === alt.time
                              ? "border-primary bg-primary/10 shadow-2xs"
                              : "border-border/80 bg-card hover:border-border"
                          }`}
                        >
                          <div className="space-y-1">
                            <div className="flex items-center gap-2">
                              <span className="font-semibold text-sm text-foreground">{alt.time}</span>
                              <span className="text-xs text-muted-foreground">· {alt.area}</span>
                            </div>
                            <span className="inline-block rounded-md bg-muted px-2 py-0.5 text-[10px] font-medium text-foreground/80">
                              {alt.reason}
                            </span>
                          </div>

                          <Button
                            size="sm"
                            variant={selectedAlternative?.time === alt.time ? "default" : "outline"}
                            className="h-8 text-xs"
                          >
                            Select
                          </Button>
                        </div>
                      ))}
                    </div>

                    {/* Secondary Actions */}
                    <div className="flex flex-col sm:flex-row items-center gap-3 pt-2">
                      <Button
                        variant="outline"
                        onClick={() => navigate("/assistant")}
                        className="h-10 w-full sm:w-auto text-xs"
                      >
                        Join Waitlist Instead
                      </Button>
                      <Button
                        disabled={!selectedAlternative}
                        onClick={() => setStep(3)}
                        className="h-10 w-full sm:flex-1 text-xs font-medium shadow-xs"
                      >
                        Continue with Selected Slot
                        <ArrowRight className="size-3.5 ml-1.5" />
                      </Button>
                    </div>
                  </div>
                )}
              </motion.div>
            )}

            {/* STEP 3: YOUR DETAILS */}
            {step === 3 && (
              <motion.div
                initial={shouldReduceMotion ? { opacity: 1 } : { opacity: 0, y: 12 }}
                animate={{ opacity: 1, y: 0 }}
                className="rounded-2xl border border-border/80 bg-card p-6 shadow-xs space-y-6"
              >
                <div>
                  <h2 className="text-xl font-semibold tracking-tight text-foreground">
                    Guest Details
                  </h2>
                  <p className="text-xs sm:text-sm text-muted-foreground leading-relaxed pt-1">
                    Enter your contact information for reservation confirmation and table management.
                  </p>
                </div>

                <form onSubmit={handleSubmit(onDetailsSubmit)} className="space-y-4">
                  {/* Full Name */}
                  <div className="space-y-1.5">
                    <label htmlFor="fullName" className="text-xs font-medium text-foreground">
                      Full Name <span className="text-destructive">*</span>
                    </label>
                    <input
                      id="fullName"
                      {...register("fullName")}
                      placeholder="e.g. Priya Sharma"
                      className="h-10 w-full rounded-xl border border-border/80 bg-muted/20 px-3.5 text-sm text-foreground placeholder:text-muted-foreground focus:border-ring focus:outline-none focus:ring-2 focus:ring-ring/20"
                    />
                    {errors.fullName && (
                      <p className="text-[11px] text-destructive">{errors.fullName.message}</p>
                    )}
                  </div>

                  {/* Phone & Email */}
                  <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
                    <div className="space-y-1.5">
                      <label htmlFor="phone" className="text-xs font-medium text-foreground">
                        Phone (Optional)
                      </label>
                      <input
                        id="phone"
                        {...register("phone")}
                        placeholder="+91 98765 43210"
                        className="h-10 w-full rounded-xl border border-border/80 bg-muted/20 px-3.5 text-sm text-foreground placeholder:text-muted-foreground focus:border-ring focus:outline-none focus:ring-2 focus:ring-ring/20"
                      />
                      {errors.phone && (
                        <p className="text-[11px] text-destructive">{errors.phone.message}</p>
                      )}
                    </div>

                    <div className="space-y-1.5">
                      <label htmlFor="email" className="text-xs font-medium text-foreground">
                        Email (Optional)
                      </label>
                      <input
                        id="email"
                        {...register("email")}
                        placeholder="priya@example.com"
                        className="h-10 w-full rounded-xl border border-border/80 bg-muted/20 px-3.5 text-sm text-foreground placeholder:text-muted-foreground focus:border-ring focus:outline-none focus:ring-2 focus:ring-ring/20"
                      />
                      {errors.email && (
                        <p className="text-[11px] text-destructive">{errors.email.message}</p>
                      )}
                    </div>
                  </div>

                  {/* Special Requests */}
                  <div className="space-y-1.5">
                    <div className="flex items-center justify-between text-xs">
                      <label htmlFor="specialRequests" className="font-medium text-foreground">
                        Special Requests & Dietary Notes
                      </label>
                      <span className="text-[11px] text-muted-foreground">
                        {specialRequestsText.length}/200
                      </span>
                    </div>
                    <textarea
                      id="specialRequests"
                      rows={3}
                      {...register("specialRequests")}
                      placeholder="e.g. Quiet corner table requested, anniversary celebration, dairy allergy..."
                      className="w-full resize-none rounded-xl border border-border/80 bg-muted/20 p-3 text-sm text-foreground placeholder:text-muted-foreground focus:border-ring focus:outline-none focus:ring-2 focus:ring-ring/20"
                    />
                  </div>

                  {/* Opt-in Memory Consent (Unchecked by default) */}
                  <div className="flex items-start gap-3 rounded-xl border border-border/80 bg-muted/20 p-3.5">
                    <input
                      id="saveProfileConsent"
                      type="checkbox"
                      {...register("saveProfileConsent")}
                      className="mt-0.5 size-4 accent-primary cursor-pointer"
                    />
                    <div className="space-y-0.5 text-xs">
                      <label htmlFor="saveProfileConsent" className="font-medium text-foreground cursor-pointer">
                        Remember preferences for future visits
                      </label>
                      <p className="text-[11px] text-muted-foreground leading-relaxed">
                        Opt-in memory stores your dietary notes and preferred seating in an unlinked profile. You can export or delete your profile anytime in Privacy settings.
                      </p>
                    </div>
                  </div>

                  {/* Submit / Back Actions */}
                  <div className="flex items-center gap-3 pt-3">
                    <Button
                      type="button"
                      variant="outline"
                      onClick={() => setStep(2)}
                      className="h-11 text-xs"
                    >
                      <ArrowLeft className="size-3.5 mr-1.5" />
                      Back
                    </Button>
                    <Button
                      type="submit"
                      className="h-11 flex-1 text-xs font-medium shadow-xs"
                    >
                      {isDepositRequired ? "Proceed to Deposit Hold" : "Confirm Reservation"}
                      <ArrowRight className="size-3.5 ml-1.5" />
                    </Button>
                  </div>
                </form>
              </motion.div>
            )}

            {/* STEP 4: DEPOSIT HOLD (Required for parties >= 8) */}
            {step === 4 && (
              <motion.div
                initial={shouldReduceMotion ? { opacity: 1 } : { opacity: 0, y: 12 }}
                animate={{ opacity: 1, y: 0 }}
                className="rounded-2xl border border-border/80 bg-card p-6 shadow-xs space-y-6"
              >
                <div className="flex items-center gap-2 text-primary">
                  <CreditCard className="size-5" />
                  <h2 className="text-xl font-semibold tracking-tight text-foreground">
                    Group Deposit Required
                  </h2>
                </div>

                <p className="text-xs sm:text-sm text-muted-foreground leading-relaxed">
                  Large dining parties of 8 or more require a deposit hold of ₹2,000 to protect table capacity. The table is being held temporarily.
                </p>

                {/* Simulated Payment Banner */}
                <SimulatedBanner
                  type="payment"
                  message="SIMULATED PAYMENT · No real cards or financial transactions"
                />

                {/* Deposit Summary Card */}
                <div className="rounded-xl border border-border/80 bg-muted/20 p-4 space-y-4">
                  <div className="flex items-center justify-between">
                    <span className="text-xs text-muted-foreground">Deposit Hold Amount</span>
                    <span className="text-lg font-bold text-foreground">
                      <Money amountMinor={200000} currency="INR" />
                    </span>
                  </div>

                  {/* Hold Countdown */}
                  <div className="flex items-center justify-between border-t border-border/60 pt-3 text-xs">
                    <span className="text-muted-foreground">Hold Expiration</span>
                    <CountdownChip expiresAt={depositDeadline.toISOString()} />
                  </div>

                  {/* Refund Policy */}
                  <div className="rounded-lg bg-card border border-border/70 p-3 text-[11px] text-muted-foreground space-y-1">
                    <strong className="text-foreground block">Deposit Refund Policy:</strong>
                    <p>• 100% refund for cancellations up to 24 hours before dining.</p>
                    <p>• 50% refund for cancellations between 12 and 24 hours.</p>
                    <p>• Non-refundable for cancellations under 12 hours.</p>
                  </div>
                </div>

                {/* Checkout Trigger */}
                <div className="flex flex-col sm:flex-row items-center gap-3 pt-2">
                  <Button
                    variant="outline"
                    onClick={() => {
                      const demoToken = `sim_dep_${Date.now()}`
                      navigate(`/checkout/${demoToken}`)
                    }}
                    className="h-11 w-full sm:flex-1 text-xs"
                  >
                    Open Checkout Page
                  </Button>
                  <Button
                    onClick={() => {
                      setDepositPaid(true)
                      const code = `TT-${Math.floor(100000 + Math.random() * 900000)}`
                      setConfirmationCode(code)
                      setStep(5)
                    }}
                    className="h-11 w-full sm:flex-1 text-xs font-medium shadow-xs"
                  >
                    Simulate Deposit Approval
                    <ArrowRight className="size-3.5 ml-1.5" />
                  </Button>
                </div>
              </motion.div>
            )}

            {/* STEP 5: CONFIRMATION SUCCESS */}
            {step === 5 && (
              <motion.div
                initial={shouldReduceMotion ? { opacity: 1 } : { opacity: 0, y: 12 }}
                animate={{ opacity: 1, y: 0 }}
                className="rounded-2xl border border-border/80 bg-card p-6 sm:p-8 shadow-xs space-y-6"
              >
                <div className="flex items-center gap-3 text-emerald-600 dark:text-emerald-400">
                  <div className="flex size-12 items-center justify-center rounded-2xl bg-emerald-500/10">
                    <CheckCircle2 className="size-6 text-emerald-600 dark:text-emerald-400" />
                  </div>
                  <div>
                    <h2 className="text-xl sm:text-2xl font-semibold tracking-tight text-foreground">
                      Reservation Confirmed!
                    </h2>
                    <p className="text-xs text-muted-foreground">
                      A table has been held and confirmed under your name.
                    </p>
                  </div>
                </div>

                {/* Confirmation Code Block */}
                <div className="space-y-1.5">
                  <span className="text-xs font-medium text-muted-foreground">
                    Your Confirmation Code:
                  </span>
                  <CodeBlock code={confirmationCode} label="Confirmation Code" />
                </div>

                {/* Booking Summary Card */}
                <div className="rounded-xl border border-border/80 bg-muted/20 p-4 space-y-3 text-xs">
                  <div className="flex items-center justify-between pb-2 border-b border-border/60">
                    <span className="text-muted-foreground">Restaurant</span>
                    <strong className="text-foreground">The Saffron Nebula Bistro</strong>
                  </div>
                  <div className="grid grid-cols-2 gap-3">
                    <div>
                      <span className="text-muted-foreground block text-[11px]">Guest Name</span>
                      <strong className="text-foreground">{guestName || "Priya Sharma"}</strong>
                    </div>
                    <div>
                      <span className="text-muted-foreground block text-[11px]">Party Size</span>
                      <strong className="text-foreground">{partySize} Guests</strong>
                    </div>
                    <div>
                      <span className="text-muted-foreground block text-[11px]">Date & Time</span>
                      <strong className="text-foreground">{selectedDate} at {selectedAlternative?.time || selectedTime}</strong>
                    </div>
                    <div>
                      <span className="text-muted-foreground block text-[11px]">Seating Area</span>
                      <strong className="text-foreground capitalize">{selectedAlternative?.area || selectedArea}</strong>
                    </div>
                  </div>
                </div>

                {/* Action Buttons: Add to Calendar & Manage */}
                <div className="flex flex-col sm:flex-row items-center gap-3 pt-2">
                  <Button
                    variant="outline"
                    onClick={handleDownloadCalendar}
                    className="h-11 w-full sm:flex-1 text-xs"
                  >
                    <Download className="size-4 mr-2" />
                    Add to Calendar (.ics)
                  </Button>
                  <Button
                    asChild
                    className="h-11 w-full sm:flex-1 text-xs font-medium shadow-xs"
                  >
                    <Link to={`/manage?code=${confirmationCode}`}>
                      Manage Reservation
                      <ArrowRight className="size-3.5 ml-1.5" />
                    </Link>
                  </Button>
                </div>
              </motion.div>
            )}
          </div>

          {/* Sticky Summary Sidebar on Desktop */}
          <aside className="space-y-4">
            <div className="sticky top-20 rounded-2xl border border-border/80 bg-card p-5 shadow-xs space-y-4">
              <h3 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
                Reservation Summary
              </h3>

              <div className="space-y-3 text-xs">
                <div className="flex items-center justify-between">
                  <span className="text-muted-foreground">Date:</span>
                  <span className="font-medium text-foreground">{selectedDate}</span>
                </div>
                <div className="flex items-center justify-between">
                  <span className="text-muted-foreground">Time:</span>
                  <span className="font-medium text-foreground">{selectedAlternative?.time || selectedTime}</span>
                </div>
                <div className="flex items-center justify-between">
                  <span className="text-muted-foreground">Party:</span>
                  <span className="font-medium text-foreground">{partySize} {partySize === 1 ? "Guest" : "Guests"}</span>
                </div>
                <div className="flex items-center justify-between">
                  <span className="text-muted-foreground">Seating:</span>
                  <span className="font-medium text-foreground capitalize">{selectedAlternative?.area || selectedArea}</span>
                </div>

                {isDepositRequired && (
                  <div className="flex items-center justify-between border-t border-border/60 pt-2 font-semibold">
                    <span>Deposit:</span>
                    <Money amountMinor={200000} currency="INR" />
                  </div>
                )}
              </div>

              <div className="border-t border-border/60 pt-3 text-[11px] text-muted-foreground leading-relaxed">
                <p>Need voice assistance? You can also book by talking to our assistant.</p>
                <Link to="/assistant" className="text-primary hover:underline font-medium inline-block pt-1">
                  Launch Voice Assistant →
                </Link>
              </div>
            </div>
          </aside>
        </div>
      </div>
    </div>
  )
}
