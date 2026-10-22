import React, { useState, useEffect, useRef } from "react"
import { useNavigate } from "react-router-dom"
import { motion, AnimatePresence, useReducedMotion } from "framer-motion"
import {
  Mic,
  Send,
  Volume2,
  VolumeX,
  Square,
  Sparkles,
  RotateCcw,
  Activity,
  ChevronRight,
  X,
  AlertCircle,
  CheckCircle2,
  UtensilsCrossed,
  ShieldAlert,
} from "lucide-react"
import { toast } from "sonner"
import { Button } from "@/components/ui/button"
import { CardRenderer } from "@/cards/registry"
import type { TableTalkToolName, ToolResponseEnvelope } from "@/cards/types"
import type {
  CardEventHandlers,
  AcceptOptionEvent,
  HoldOptionEvent,
  SelectAlternativeEvent,
  CancelBookingEvent,
  CancelOrderEvent,
  OpenCheckoutEvent,
  LeaveWaitlistEvent,
  CopyCodeEvent,
} from "@/cards/events"
import {
  sendAssistantMessage,
  getToolActivityLabel,
  type AssistantCardItem,
  type LiveActivityItem,
} from "@/lib/assistantService"
import {
  SpeechRecognitionController,
  SpeechSynthesisController,
  isSpeechRecognitionSupported,
} from "@/lib/speech"

export interface Message {
  id: string
  sender: "user" | "assistant"
  text: string
  timestamp: string
  toolActivities?: Array<{
    toolName: TableTalkToolName | string
    label: string
  }>
  card?: {
    toolName: TableTalkToolName
    envelope: ToolResponseEnvelope<any>
  }
  cards?: AssistantCardItem[]
  error?: {
    message: string
    canRetry: boolean
  }
}

const SUGGESTION_CHIPS = [
  {
    display: "Table for 4 Friday at 7:30",
    alias: "Table for 4 tomorrow at 7:30",
    query: "Table for 4 tomorrow at 7:30",
  },
  {
    display: "Vegan options without peanuts",
    query: "Vegan options without peanuts",
  },
  {
    display: "Order pickup",
    query: "Order pickup",
  },
  {
    display: "Join the waitlist",
    query: "Join the waitlist",
  },
]

export function AssistantPage() {
  const navigate = useNavigate()
  const shouldReduceMotion = useReducedMotion()

  // Conversation state
  const [messages, setMessages] = useState<Message[]>([])
  const [inputText, setInputText] = useState("")
  const [isThinking, setIsThinking] = useState(false)
  const [activeToolLabel, setActiveToolLabel] = useState<string | null>(null)
  const [liveActivity, setLiveActivity] = useState<LiveActivityItem[]>([])
  const [showActivityDrawer, setShowActivityDrawer] = useState(false)
  const [showScrollBottomPill, setShowScrollBottomPill] = useState(false)
  const [isFallbackMode, setIsFallbackMode] = useState(false)
  const [activeProvider, setActiveProvider] = useState<string>("groq")

  // In-memory token only (never localStorage)
  const inMemoryProfileTokenRef = useRef<string | null>(null)

  // Speech Controllers
  const speechRecRef = useRef<SpeechRecognitionController | null>(null)
  const speechSynthRef = useRef<SpeechSynthesisController | null>(null)
  const [isListening, setIsListening] = useState(false)
  const [isSpeaking, setIsSpeaking] = useState(false)
  const [isVoiceMuted, setIsVoiceMuted] = useState(false)
  const [speechRecSupported, setSpeechRecSupported] = useState(true)

  // Confirmation dialog for destructive actions
  const [confirmDialog, setConfirmDialog] = useState<{
    open: boolean
    title: string
    description: string
    confirmLabel: string
    onConfirm: () => void
  } | null>(null)

  const messagesEndRef = useRef<HTMLDivElement | null>(null)
  const chatScrollContainerRef = useRef<HTMLDivElement | null>(null)
  const textareaRef = useRef<HTMLTextAreaElement | null>(null)

  // Initialize Speech on client
  useEffect(() => {
    setSpeechRecSupported(isSpeechRecognitionSupported())
    speechRecRef.current = new SpeechRecognitionController()
    speechSynthRef.current = new SpeechSynthesisController()

    return () => {
      speechRecRef.current?.stop()
      speechSynthRef.current?.stop()
    }
  }, [])

  // Auto scroll to bottom
  const scrollToBottom = (behavior: ScrollBehavior = "smooth") => {
    messagesEndRef.current?.scrollIntoView({
      behavior: shouldReduceMotion ? "auto" : behavior,
    })
    setShowScrollBottomPill(false)
  }

  useEffect(() => {
    scrollToBottom()
  }, [messages, isThinking])

  // Track scroll position to show "new messages" button
  const handleScroll = () => {
    const el = chatScrollContainerRef.current
    if (!el) return
    const isNearBottom = el.scrollHeight - el.scrollTop - el.clientHeight < 120
    setShowScrollBottomPill(!isNearBottom && messages.length > 0)
  }

  // Auto-resize textarea
  const handleTextareaInput = (e: React.ChangeEvent<HTMLTextAreaElement>) => {
    setInputText(e.target.value)
    if (textareaRef.current) {
      textareaRef.current.style.height = "auto"
      textareaRef.current.style.height = `${Math.min(textareaRef.current.scrollHeight, 140)}px`
    }
  }

  // Send a user prompt to the assistant orchestrator
  const handleSend = async (textToSend?: string) => {
    const text = (textToSend ?? inputText).trim()
    if (!text || isThinking) return

    // Stop listening/speaking if in progress
    speechRecRef.current?.stop()
    setIsListening(false)
    speechSynthRef.current?.stop()
    setIsSpeaking(false)

    setInputText("")
    if (textareaRef.current) {
      textareaRef.current.style.height = "auto"
    }

    const timestamp = new Date().toLocaleTimeString([], {
      hour: "2-digit",
      minute: "2-digit",
    })

    const userMsg: Message = {
      id: `user-${Date.now()}`,
      sender: "user",
      text,
      timestamp,
    }

    const currentHistory = messages.map((m) => ({
      role: m.sender,
      content: m.text,
    }))

    setMessages((prev) => [...prev, userMsg])
    setIsThinking(true)
    setActiveToolLabel("Checking restaurant capacity...")

    try {
      const response = await sendAssistantMessage(
        text,
        inMemoryProfileTokenRef.current,
        currentHistory
      )

      setIsThinking(false)
      setActiveToolLabel(null)

      if (response.fallbackMode) {
        setIsFallbackMode(true)
      }
      if (response.provider) {
        setActiveProvider(response.provider)
      }

      // Compute tool activity chips derived from response
      const toolActivities =
        response.cards && response.cards.length > 0
          ? response.cards.map((c) => ({
              toolName: c.toolName,
              label: getToolActivityLabel(c.toolName),
            }))
          : response.toolName
            ? [{ toolName: response.toolName, label: getToolActivityLabel(response.toolName) }]
            : []

      const assistantMsg: Message = {
        id: `asst-${Date.now()}`,
        sender: "assistant",
        text: response.spokenText,
        timestamp: new Date().toLocaleTimeString([], {
          hour: "2-digit",
          minute: "2-digit",
        }),
        toolActivities,
        cards: response.cards,
        card: response.envelope && response.toolName
          ? {
              toolName: response.toolName,
              envelope: response.envelope,
            }
          : undefined,
      }

      setMessages((prev) => [...prev, assistantMsg])

      // Record in Live Activity stream
      if (response.cards && response.cards.length > 0) {
        for (const c of response.cards) {
          setLiveActivity((prev) => [
            {
              id: `act-${Date.now()}-${Math.random()}`,
              timestamp: new Date().toLocaleTimeString([], {
                hour: "2-digit",
                minute: "2-digit",
                second: "2-digit",
              }),
              toolName: c.toolName,
              status: response.status,
              durationMs: response.durationMs,
              summary: `Dispatched tool: ${c.toolName} (${response.durationMs}ms)`,
            },
            ...prev,
          ])
        }
      } else if (response.toolName) {
        setLiveActivity((prev) => [
          {
            id: `act-${Date.now()}`,
            timestamp: new Date().toLocaleTimeString([], {
              hour: "2-digit",
              minute: "2-digit",
              second: "2-digit",
            }),
            toolName: response.toolName!,
            status: response.status,
            durationMs: response.durationMs,
            summary: `Dispatched tool: ${response.toolName} (${response.durationMs}ms)`,
          },
          ...prev,
        ])
      }

      // Read reply aloud if not muted
      if (!isVoiceMuted && speechSynthRef.current?.isSupported) {
        setIsSpeaking(true)
        speechSynthRef.current.speak(response.spokenText, {
          onStart: () => setIsSpeaking(true),
          onEnd: () => setIsSpeaking(false),
          onError: () => setIsSpeaking(false),
        })
      }
    } catch {
      setIsThinking(false)
      setActiveToolLabel(null)

      const errorMsg: Message = {
        id: `err-${Date.now()}`,
        sender: "assistant",
        text: "I couldn't complete that request right now. You can try again or rephrase your question.",
        timestamp: new Date().toLocaleTimeString([], {
          hour: "2-digit",
          minute: "2-digit",
        }),
        error: {
          message: "Connection failed. Please check your network and retry.",
          canRetry: true,
        },
      }

      setMessages((prev) => [...prev, errorMsg])
    }
  }

  // Voice recording toggle
  const toggleListening = () => {
    if (isListening) {
      speechRecRef.current?.stop()
      setIsListening(false)
      return
    }

    if (!speechRecSupported) {
      toast.info("Microphone recognition is not supported in this browser. Please type your query below.")
      return
    }

    speechSynthRef.current?.stop()
    setIsSpeaking(false)

    const started = speechRecRef.current?.start({
      onStart: () => setIsListening(true),
      onTranscript: (transcript, isFinal) => {
        setInputText(transcript)
        if (isFinal) {
          setIsListening(false)
          handleSend(transcript)
        }
      },
      onError: (err) => {
        setIsListening(false)
        toast.error(`Microphone input error: ${err}`)
      },
      onEnd: () => setIsListening(false),
    })

    if (!started) {
      setIsListening(false)
    }
  }

  // Voice mute toggle
  const toggleVoiceMute = () => {
    const next = !isVoiceMuted
    setIsVoiceMuted(next)
    speechSynthRef.current?.setMuted(next)
    if (next) {
      speechSynthRef.current?.stop()
      setIsSpeaking(false)
      toast.info("Voice narration muted")
    } else {
      toast.info("Voice narration enabled")
    }
  }

  // Stop assistant speaking immediately
  const handleStopSpeaking = () => {
    speechSynthRef.current?.stop()
    setIsSpeaking(false)
  }

  // Card Event Listeners: Cards emit events only; this page turns them into new chat turns
  const cardEventHandlers: CardEventHandlers = {
    onAcceptOption: (event: AcceptOptionEvent) => {
      handleSend(event.optionId ? `Accept option ${event.optionId}` : "Accept booking offer")
    },
    onHoldOption: (event: HoldOptionEvent) => {
      handleSend(`Hold option ${event.optionId} for 5 minutes`)
    },
    onSelectAlternative: (event: SelectAlternativeEvent) => {
      handleSend(
        `Check availability for ${event.date} at ${event.time} in ${event.area}`
      )
    },
    onCancelBooking: (event: CancelBookingEvent) => {
      // Destructive action: require explicit confirmation dialog
      setConfirmDialog({
        open: true,
        title: "Cancel Reservation?",
        description: `Are you sure you want to cancel booking ${event.confirmationCode}? Any eligible deposit will be processed according to the cancellation policy.`,
        confirmLabel: "Cancel Booking",
        onConfirm: () => {
          setConfirmDialog(null)
          handleSend(`Cancel booking ${event.confirmationCode}`)
        },
      })
    },
    onCancelOrder: (event: CancelOrderEvent) => {
      // Destructive action: require explicit confirmation dialog
      setConfirmDialog({
        open: true,
        title: "Cancel Pickup Order?",
        description: `Are you sure you want to cancel pickup order ${event.orderCode}?`,
        confirmLabel: "Cancel Order",
        onConfirm: () => {
          setConfirmDialog(null)
          handleSend(`Cancel order ${event.orderCode}`)
        },
      })
    },
    onLeaveWaitlist: (event: LeaveWaitlistEvent) => {
      // Destructive action: require explicit confirmation dialog
      setConfirmDialog({
        open: true,
        title: "Leave Waitlist?",
        description:
          "Are you sure you want to forfeit your current position in the waitlist?",
        confirmLabel: "Leave Waitlist",
        onConfirm: () => {
          setConfirmDialog(null)
          handleSend(event.waitlistId ? `Leave waitlist ${event.waitlistId}` : "Leave waitlist")
        },
      })
    },
    onOpenCheckout: (event: OpenCheckoutEvent) => {
      // Direct navigation to dedicated checkout page; cards never process payments
      if (event.checkoutUrl.startsWith("/checkout/")) {
        navigate(event.checkoutUrl)
      } else {
        const tokenMatch = event.checkoutUrl.match(/\/checkout\/([^/?#]+)/)
        if (tokenMatch) {
          navigate(`/checkout/${tokenMatch[1]}`)
        } else {
          window.location.href = event.checkoutUrl
        }
      }
    },
    onCopyCode: (event: CopyCodeEvent) => {
      if (typeof navigator !== "undefined" && navigator.clipboard) {
        navigator.clipboard.writeText(event.code)
        toast.success(`Copied ${event.label}: ${event.code}`)
      }
    },
  }

  // Mic state determination
  const micState: "idle" | "listening" | "thinking" | "speaking" = isListening
    ? "listening"
    : isThinking
      ? "thinking"
      : isSpeaking
        ? "speaking"
        : "idle"

  return (
    <div className="relative flex h-[calc(100vh-3.5rem)] flex-col overflow-hidden bg-background">
      {/* Invisible screen reader announcer */}
      <div className="sr-only" aria-live="polite" aria-atomic="true">
        {messages.length > 0 &&
          messages[messages.length - 1].sender === "assistant" &&
          messages[messages.length - 1].text}
      </div>

      {/* Top Bar inside assistant page: Status, Badges, and Controls */}
      <div className="shrink-0 border-b border-border/70 bg-card/40 px-4 py-2.5 backdrop-blur-xs">
        <div className="mx-auto flex max-w-5xl items-center justify-between gap-3">
          <div className="flex items-center gap-2.5">
            <span
              className={`size-2 rounded-full ${
                isListening
                  ? "bg-red-500 animate-ping"
                  : isSpeaking
                    ? "bg-emerald-500 animate-pulse"
                    : isThinking
                      ? "bg-amber-500 animate-pulse"
                      : "bg-primary"
              }`}
            />
            <span className="text-xs font-semibold text-foreground tracking-tight">
              {isListening
                ? "Listening..."
                : isSpeaking
                  ? "Speaking..."
                  : isThinking
                    ? "Thinking..."
                    : "TableTalk Voice Assistant"}
            </span>

            {/* Provider and Demo mode indicator badge */}
            {isFallbackMode ? (
              <span className="inline-flex items-center gap-1 rounded-full border border-amber-500/30 bg-amber-500/10 px-2 py-0.5 text-[10px] font-semibold text-amber-600 dark:text-amber-400">
                <Sparkles className="size-2.5" />
                Demo mode
              </span>
            ) : (
              <span className="hidden sm:inline-flex items-center gap-1 rounded-full border border-border bg-muted/60 px-2 py-0.5 text-[10px] font-medium text-muted-foreground uppercase tracking-wider">
                {activeProvider}
              </span>
            )}
          </div>

          <div className="flex items-center gap-2">
            {/* Stop speaking button */}
            {isSpeaking && (
              <Button
                variant="outline"
                size="sm"
                onClick={handleStopSpeaking}
                className="h-8 gap-1.5 text-xs font-medium text-destructive hover:bg-destructive/10"
                aria-label="Stop Speaking"
              >
                <Square className="size-3 fill-destructive" />
                <span>Stop</span>
              </Button>
            )}

            {/* Voice narration toggle */}
            <Button
              variant="ghost"
              size="sm"
              onClick={toggleVoiceMute}
              className="h-8 w-8 p-0 text-muted-foreground hover:text-foreground"
              aria-label={isVoiceMuted ? "Enable Voice Narration" : "Mute Voice Narration"}
              title={isVoiceMuted ? "Enable Voice Narration" : "Mute Voice Narration"}
            >
              {isVoiceMuted ? (
                <VolumeX className="size-4" />
              ) : (
                <Volume2 className="size-4" />
              )}
            </Button>

            {/* Toggle Live Activity drawer (Desktop & Mobile) */}
            <Button
              variant="outline"
              size="sm"
              onClick={() => setShowActivityDrawer((v) => !v)}
              className="h-8 gap-1.5 text-xs text-muted-foreground hover:text-foreground"
              aria-expanded={showActivityDrawer}
              aria-label="Toggle Live Activity panel"
            >
              <Activity className="size-3.5 text-primary" />
              <span className="hidden sm:inline">Live Activity</span>
              {liveActivity.length > 0 && (
                <span className="flex size-4 items-center justify-center rounded-full bg-primary/10 text-[10px] font-semibold text-primary">
                  {liveActivity.length}
                </span>
              )}
            </Button>
          </div>
        </div>
      </div>

      {/* Main Content Area: Chat Column + Slim Activity Panel */}
      <div className="relative flex flex-1 overflow-hidden">
        {/* Conversation Column (centered, max 720px) */}
        <div
          ref={chatScrollContainerRef}
          onScroll={handleScroll}
          className="flex-1 overflow-y-auto px-4 py-6 scroll-smooth"
        >
          <div className="mx-auto max-w-[720px] space-y-6">
            {/* Empty State when no messages */}
            {messages.length === 0 && (
              <motion.div
                initial={shouldReduceMotion ? { opacity: 1 } : { opacity: 0, y: 12 }}
                animate={{ opacity: 1, y: 0 }}
                transition={{ duration: 0.25 }}
                className="py-8 sm:py-16 text-center space-y-6"
              >
                <div className="inline-flex size-14 items-center justify-center rounded-2xl border border-border/80 bg-card shadow-xs">
                  <UtensilsCrossed className="size-7 text-primary" />
                </div>

                <div className="space-y-2 max-w-md mx-auto">
                  <h1 className="text-2xl sm:text-3xl font-semibold tracking-tight text-foreground">
                    How can I help you dine today?
                  </h1>
                  <p className="text-sm text-muted-foreground leading-relaxed">
                    Check table availability, browse dishes with allergen notices, or order takeout pickup.
                  </p>
                </div>

                {/* 4 Suggestion Chips (44px touch targets) */}
                <div className="grid grid-cols-1 sm:grid-cols-2 gap-2.5 max-w-lg mx-auto pt-2 text-left">
                  {SUGGESTION_CHIPS.map((chip) => (
                    <button
                      key={chip.display}
                      type="button"
                      onClick={() => handleSend(chip.query)}
                      className="group flex min-h-[44px] items-center justify-between rounded-xl border border-border/80 bg-card p-3 text-xs sm:text-sm font-medium text-foreground/90 shadow-2xs hover:border-primary/50 hover:bg-muted/50 transition-all focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                    >
                      <span className="truncate pr-2">{chip.display}</span>
                      {chip.alias && (
                        <span className="sr-only">{chip.alias}</span>
                      )}
                      <ChevronRight className="size-4 text-muted-foreground shrink-0 group-hover:text-primary transition-colors" />
                    </button>
                  ))}
                </div>
              </motion.div>
            )}

            {/* Message List */}
            {messages.map((message) => (
              <motion.div
                key={message.id}
                initial={shouldReduceMotion ? { opacity: 1 } : { opacity: 0, y: 8 }}
                animate={{ opacity: 1, y: 0 }}
                transition={{ duration: 0.2 }}
                className={`flex flex-col ${
                  message.sender === "user" ? "items-end" : "items-start"
                }`}
              >
                {/* Tool Activity Chips (derived from response) */}
                {message.sender === "assistant" && message.toolActivities && message.toolActivities.length > 0 && (
                  <div className="mb-1.5 flex flex-wrap gap-1.5 pl-1">
                    {message.toolActivities.map((act, i) => (
                      <span
                        key={i}
                        className="inline-flex items-center gap-1.5 rounded-full border border-primary/25 bg-primary/10 px-2.5 py-0.5 text-[11px] font-medium text-primary shadow-2xs"
                      >
                        <Sparkles className="size-3" />
                        {act.label}
                      </span>
                    ))}
                  </div>
                )}

                {/* Spoken Text Bubble */}
                <div
                  className={`relative max-w-[85%] sm:max-w-[75%] rounded-2xl px-4 py-3 text-sm leading-relaxed ${
                    message.sender === "user"
                      ? "bg-primary text-primary-foreground font-medium rounded-br-xs shadow-xs"
                      : "border border-border/80 bg-card text-foreground rounded-bl-xs shadow-xs"
                  }`}
                >
                  <p className="whitespace-pre-wrap">{message.text}</p>
                  <span
                    className={`block pt-1.5 text-[10px] ${
                      message.sender === "user"
                        ? "text-primary-foreground/75 text-right"
                        : "text-muted-foreground"
                    }`}
                  >
                    {message.timestamp}
                  </span>
                </div>

                {/* Attached Interactive Cards (rendered from CardRegistry) */}
                {message.cards && message.cards.length > 0 ? (
                  message.cards.map((cardItem, idx) => (
                    <motion.div
                      key={idx}
                      initial={shouldReduceMotion ? { opacity: 1 } : { opacity: 0, scale: 0.98 }}
                      animate={{ opacity: 1, scale: 1 }}
                      transition={{ duration: 0.25, ease: "easeOut" }}
                      className="w-full max-w-[640px] pt-3"
                    >
                      <div className="overflow-hidden rounded-xl border border-border/80 bg-card shadow-xs">
                        <CardRenderer
                          toolName={cardItem.toolName}
                          envelope={cardItem.envelope}
                          events={cardEventHandlers}
                        />
                      </div>
                    </motion.div>
                  ))
                ) : message.card ? (
                  <motion.div
                    initial={
                      shouldReduceMotion ? { opacity: 1 } : { opacity: 0, scale: 0.98 }
                    }
                    animate={{ opacity: 1, scale: 1 }}
                    transition={{ duration: 0.25, ease: "easeOut" }}
                    className="w-full max-w-[640px] pt-3"
                  >
                    <div className="overflow-hidden rounded-xl border border-border/80 bg-card shadow-xs">
                      <CardRenderer
                        toolName={message.card.toolName}
                        envelope={message.card.envelope}
                        events={cardEventHandlers}
                      />
                    </div>
                  </motion.div>
                ) : null}

                {/* Inline friendly error with retry */}
                {message.error && (
                  <div className="mt-2 flex items-center gap-2 rounded-lg border border-destructive/30 bg-destructive/5 px-3 py-2 text-xs text-destructive">
                    <AlertCircle className="size-4 shrink-0" />
                    <span>{message.error.message}</span>
                    {message.error.canRetry && (
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => handleSend(messages[messages.length - 2]?.text)}
                        className="ml-auto h-7 px-2 text-xs text-destructive hover:bg-destructive/10"
                      >
                        <RotateCcw className="size-3 mr-1" />
                        Retry
                      </Button>
                    )}
                  </div>
                )}
              </motion.div>
            ))}

            {/* Thinking / Tool Activity Indicator */}
            {isThinking && (
              <motion.div
                initial={shouldReduceMotion ? { opacity: 1 } : { opacity: 0, y: 6 }}
                animate={{ opacity: 1, y: 0 }}
                className="flex items-center gap-2.5 text-xs text-muted-foreground py-2"
              >
                <div className="flex size-7 items-center justify-center rounded-lg bg-card border border-border/80 shadow-2xs">
                  <Sparkles className="size-3.5 text-primary animate-pulse" />
                </div>
                <div className="flex items-center gap-2">
                  <span>{activeToolLabel || "Processing request..."}</span>
                  <span className="flex space-x-1">
                    <span className="size-1 rounded-full bg-primary animate-bounce [animation-delay:-0.3s]" />
                    <span className="size-1 rounded-full bg-primary animate-bounce [animation-delay:-0.15s]" />
                    <span className="size-1 rounded-full bg-primary animate-bounce" />
                  </span>
                </div>
              </motion.div>
            )}

            <div ref={messagesEndRef} className="h-4" />
          </div>
        </div>

        {/* Desktop Collapsible Live Activity Slim Right Panel */}
        <AnimatePresence>
          {showActivityDrawer && (
            <motion.aside
              initial={shouldReduceMotion ? { opacity: 1 } : { opacity: 0, width: 0 }}
              animate={{ opacity: 1, width: 320 }}
              exit={shouldReduceMotion ? { opacity: 0 } : { opacity: 0, width: 0 }}
              transition={{ duration: 0.2 }}
              className="hidden lg:flex flex-col border-l border-border/80 bg-card/60 backdrop-blur-xs overflow-hidden shrink-0"
              aria-label="Live MCP Activity Stream"
            >
              <div className="flex items-center justify-between border-b border-border/70 p-3.5">
                <div className="flex items-center gap-2">
                  <Activity className="size-4 text-primary" />
                  <h2 className="text-xs font-semibold text-foreground tracking-tight">
                    Live Tool Activity
                  </h2>
                </div>
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => setShowActivityDrawer(false)}
                  className="h-7 w-7 p-0 text-muted-foreground hover:text-foreground"
                  aria-label="Close activity panel"
                >
                  <X className="size-3.5" />
                </Button>
              </div>

              <div className="flex-1 overflow-y-auto p-3 space-y-2.5 text-xs">
                {liveActivity.length === 0 ? (
                  <p className="text-muted-foreground text-center py-8">
                    No tools executed yet. Voice queries will stream MCP activity here.
                  </p>
                ) : (
                  liveActivity.map((act) => (
                    <div
                      key={act.id}
                      className="rounded-lg border border-border/70 bg-card p-2.5 space-y-1 shadow-2xs font-mono"
                    >
                      <div className="flex items-center justify-between text-[11px]">
                        <span className="font-semibold text-foreground">{act.toolName}</span>
                        <span className="text-muted-foreground">{act.timestamp}</span>
                      </div>
                      <div className="flex items-center justify-between text-[10px] text-muted-foreground">
                        <span className="flex items-center gap-1 text-emerald-600 dark:text-emerald-400">
                          <CheckCircle2 className="size-3" />
                          {act.status}
                        </span>
                        <span>{act.durationMs}ms</span>
                      </div>
                    </div>
                  ))
                )}
              </div>
            </motion.aside>
          )}
        </AnimatePresence>
      </div>

      {/* Mobile Drawer for Live Activity */}
      <AnimatePresence>
        {showActivityDrawer && (
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            className="lg:hidden fixed inset-0 z-50 bg-background/80 backdrop-blur-xs flex flex-col justify-end"
          >
            <motion.div
              initial={{ y: "100%" }}
              animate={{ y: 0 }}
              exit={{ y: "100%" }}
              transition={{ duration: 0.25 }}
              className="bg-card border-t border-border rounded-t-2xl max-h-[70vh] flex flex-col p-4 shadow-xl"
            >
              <div className="flex items-center justify-between pb-3 border-b border-border">
                <div className="flex items-center gap-2">
                  <Activity className="size-4 text-primary" />
                  <h2 className="text-sm font-semibold text-foreground">Live MCP Activity</h2>
                </div>
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => setShowActivityDrawer(false)}
                  className="h-8 w-8 p-0"
                  aria-label="Close"
                >
                  <X className="size-4" />
                </Button>
              </div>
              <div className="overflow-y-auto py-3 space-y-2 flex-1">
                {liveActivity.length === 0 ? (
                  <p className="text-muted-foreground text-center py-6 text-xs">
                    No tools executed yet.
                  </p>
                ) : (
                  liveActivity.map((act) => (
                    <div
                      key={act.id}
                      className="rounded-lg border border-border bg-muted/40 p-2.5 text-xs font-mono"
                    >
                      <div className="flex items-center justify-between font-semibold">
                        <span>{act.toolName}</span>
                        <span className="text-[10px] text-muted-foreground">{act.durationMs}ms</span>
                      </div>
                    </div>
                  ))
                )}
              </div>
            </motion.div>
          </motion.div>
        )}
      </AnimatePresence>

      {/* Floating "Scroll to Bottom" Pill */}
      {showScrollBottomPill && (
        <Button
          variant="outline"
          size="sm"
          onClick={() => scrollToBottom()}
          className="absolute bottom-24 right-6 z-20 h-8 rounded-full border-border bg-card px-3 shadow-md gap-1 text-xs font-medium"
        >
          <span>New messages</span>
          <ChevronRight className="size-3.5 rotate-90" />
        </Button>
      )}

      {/* Bottom Composer Bar (Sticks above tab bar, safe area) */}
      <div className="shrink-0 border-t border-border/80 bg-background/95 backdrop-blur-md px-4 py-3 pb-safe">
        <div className="mx-auto max-w-[720px] flex items-end gap-2.5">
          {/* Main Large Voice Mic Button */}
          <button
            type="button"
            onClick={toggleListening}
            className={`relative flex size-12 shrink-0 items-center justify-center rounded-2xl border transition-all outline-none focus-visible:ring-2 focus-visible:ring-ring ${
              micState === "listening"
                ? "border-red-500 bg-red-500/10 text-red-500 shadow-md ring-4 ring-red-500/20"
                : micState === "thinking"
                  ? "border-amber-500 bg-amber-500/10 text-amber-500 animate-pulse"
                  : micState === "speaking"
                    ? "border-emerald-500 bg-emerald-500/10 text-emerald-500 shadow-md ring-4 ring-emerald-500/20"
                    : "border-border/80 bg-card text-foreground hover:border-primary/50 hover:bg-muted/60 shadow-2xs"
            }`}
            aria-label={
              micState === "listening"
                ? "Stop Listening"
                : micState === "speaking"
                  ? "Assistant is Speaking"
                  : "Start Voice Input"
            }
            title={
              micState === "listening"
                ? "Listening... Click to stop"
                : "Click to speak to the voice assistant"
            }
          >
            {/* Animated Waveform Ring when listening */}
            {micState === "listening" && !shouldReduceMotion && (
              <span className="absolute -inset-1 rounded-2xl border border-red-500/40 animate-ping" />
            )}

            {/* Waveform bars when speaking */}
            {micState === "speaking" ? (
              <div className="flex items-center gap-0.5" aria-hidden="true">
                <span className="h-4 w-1 rounded-full bg-emerald-500 animate-pulse" />
                <span className="h-6 w-1 rounded-full bg-emerald-500 animate-pulse [animation-delay:0.15s]" />
                <span className="h-3 w-1 rounded-full bg-emerald-500 animate-pulse [animation-delay:0.3s]" />
              </div>
            ) : micState === "listening" ? (
              <Mic className="size-5 text-red-500 animate-pulse" />
            ) : (
              <Mic className="size-5" />
            )}
          </button>

          {/* Auto-growing Textarea Input */}
          <div className="relative flex-1 overflow-hidden rounded-xl border border-border/80 bg-card shadow-2xs focus-within:border-ring focus-within:ring-2 focus-within:ring-ring/20 transition-all">
            <textarea
              ref={textareaRef}
              rows={1}
              value={inputText}
              onChange={handleTextareaInput}
              onKeyDown={(e) => {
                if (e.key === "Enter" && !e.shiftKey) {
                  e.preventDefault()
                  handleSend()
                }
              }}
              placeholder={
                isListening
                  ? "Listening to speech..."
                  : "Ask about tables, vegan dishes, pickup..."
              }
              className="w-full resize-none bg-transparent px-3.5 py-3 text-sm text-foreground placeholder:text-muted-foreground focus:outline-none min-h-[44px] max-h-[140px]"
            />
          </div>

          {/* Send Button */}
          <Button
            type="button"
            size="default"
            disabled={!inputText.trim() || isThinking}
            onClick={() => handleSend()}
            className="size-11 shrink-0 rounded-xl p-0 shadow-xs"
            aria-label="Send message"
          >
            <Send className="size-4" />
          </Button>
        </div>
      </div>

      {/* Confirmation Dialog for Destructive Actions (WCAG accessible modal) */}
      <AnimatePresence>
        {confirmDialog && confirmDialog.open && (
          <div
            role="dialog"
            aria-modal="true"
            aria-labelledby="confirm-dialog-title"
            className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-background/80 backdrop-blur-xs"
          >
            <motion.div
              initial={shouldReduceMotion ? { opacity: 1 } : { opacity: 0, scale: 0.95 }}
              animate={{ opacity: 1, scale: 1 }}
              exit={shouldReduceMotion ? { opacity: 0 } : { opacity: 0, scale: 0.95 }}
              className="w-full max-w-md rounded-2xl border border-border bg-card p-6 shadow-xl space-y-4"
            >
              <div className="flex items-center gap-3 text-destructive">
                <div className="flex size-10 items-center justify-center rounded-xl bg-destructive/10">
                  <ShieldAlert className="size-5" />
                </div>
                <h3 id="confirm-dialog-title" className="text-base font-semibold text-foreground">
                  {confirmDialog.title}
                </h3>
              </div>

              <p className="text-xs sm:text-sm text-muted-foreground leading-relaxed">
                {confirmDialog.description}
              </p>

              <div className="flex items-center justify-end gap-2.5 pt-2">
                <Button
                  variant="outline"
                  onClick={() => setConfirmDialog(null)}
                  className="h-9 px-4 text-xs font-medium"
                >
                  Keep Booking
                </Button>
                <Button
                  variant="destructive"
                  onClick={confirmDialog.onConfirm}
                  className="h-9 px-4 text-xs font-medium shadow-xs"
                >
                  {confirmDialog.confirmLabel}
                </Button>
              </div>
            </motion.div>
          </div>
        )}
      </AnimatePresence>
    </div>
  )
}
