/**
 * Speech Recognition and Speech Synthesis abstractions for TableTalk Assistant.
 * Gracefully degrades with clear fallbacks when Web Speech API is not supported.
 */

// Types for Web Speech Recognition API
interface SpeechRecognitionEventLike {
  results: {
    [index: number]: {
      [index: number]: {
        transcript: string
      }
      isFinal: boolean
      length: number
    }
    length: number
  }
}

interface SpeechRecognitionLike {
  continuous: boolean
  interimResults: boolean
  lang: string
  start: () => void
  stop: () => void
  abort: () => void
  onstart: (() => void) | null
  onresult: ((event: SpeechRecognitionEventLike) => void) | null
  onerror: ((event: { error: string }) => void) | null
  onend: (() => void) | null
}

export function isSpeechRecognitionSupported(): boolean {
  if (typeof window === "undefined") return false
  const win = window as unknown as {
    SpeechRecognition?: new () => SpeechRecognitionLike
    webkitSpeechRecognition?: new () => SpeechRecognitionLike
  }
  return !!(win.SpeechRecognition || win.webkitSpeechRecognition)
}

export function isSpeechSynthesisSupported(): boolean {
  if (typeof window === "undefined") return false
  return "speechSynthesis" in window && typeof SpeechSynthesisUtterance !== "undefined"
}

export class SpeechRecognitionController {
  private recognition: SpeechRecognitionLike | null = null
  private isListeningState = false

  constructor() {
    if (typeof window === "undefined") return
    const win = window as unknown as {
      SpeechRecognition?: new () => SpeechRecognitionLike
      webkitSpeechRecognition?: new () => SpeechRecognitionLike
    }
    const SpeechRec = win.SpeechRecognition || win.webkitSpeechRecognition
    if (SpeechRec) {
      try {
        this.recognition = new SpeechRec()
        this.recognition.continuous = false
        this.recognition.interimResults = true
        this.recognition.lang = "en-US"
      } catch {
        this.recognition = null
      }
    }
  }

  public get isSupported(): boolean {
    return !!this.recognition
  }

  public get isListening(): boolean {
    return this.isListeningState
  }

  public start({
    onTranscript,
    onStart,
    onEnd,
    onError,
  }: {
    onTranscript: (text: string, isFinal: boolean) => void
    onStart?: () => void
    onEnd?: () => void
    onError?: (error: string) => void
  }): boolean {
    if (!this.recognition) {
      onError?.("Speech recognition is not supported in this browser.")
      return false
    }

    try {
      this.recognition.onstart = () => {
        this.isListeningState = true
        onStart?.()
      }

      this.recognition.onresult = (event: SpeechRecognitionEventLike) => {
        let transcript = ""
        let isFinal = false
        for (let i = 0; i < event.results.length; i++) {
          const res = event.results[i]
          if (res && res[0]) {
            transcript += res[0].transcript
            if (res.isFinal) isFinal = true
          }
        }
        onTranscript(transcript, isFinal)
      }

      this.recognition.onerror = (e) => {
        this.isListeningState = false
        onError?.(e.error || "Speech recognition error")
      }

      this.recognition.onend = () => {
        this.isListeningState = false
        onEnd?.()
      }

      this.recognition.start()
      return true
    } catch (err: unknown) {
      this.isListeningState = false
      onError?.(err instanceof Error ? err.message : "Failed to start microphone")
      return false
    }
  }

  public stop(): void {
    if (!this.recognition || !this.isListeningState) return
    try {
      this.recognition.stop()
    } catch {
      // Ignore abort errors
    } finally {
      this.isListeningState = false
    }
  }
}

export class SpeechSynthesisController {
  private isSpeakingState = false
  private isMutedState = false

  public get isSupported(): boolean {
    return isSpeechSynthesisSupported()
  }

  public get isSpeaking(): boolean {
    return this.isSpeakingState
  }

  public get isMuted(): boolean {
    return this.isMutedState
  }

  public setMuted(muted: boolean): void {
    this.isMutedState = muted
    if (muted) {
      this.stop()
    }
  }

  public speak(
    text: string,
    callbacks?: {
      onStart?: () => void
      onEnd?: () => void
      onError?: (err: string) => void
    }
  ): void {
    if (!this.isSupported || this.isMutedState) {
      callbacks?.onEnd?.()
      return
    }

    try {
      window.speechSynthesis.cancel() // Stop any active speech

      const utterance = new SpeechSynthesisUtterance(text)
      utterance.rate = 1.0
      utterance.pitch = 1.0

      // Pick a natural English voice if available
      const voices = window.speechSynthesis.getVoices()
      const preferred = voices.find(
        (v) => (v.lang.startsWith("en-") || v.lang === "en") && !v.name.includes("whisper")
      )
      if (preferred) {
        utterance.voice = preferred
      }

      utterance.onstart = () => {
        this.isSpeakingState = true
        callbacks?.onStart?.()
      }

      utterance.onend = () => {
        this.isSpeakingState = false
        callbacks?.onEnd?.()
      }

      utterance.onerror = (e) => {
        this.isSpeakingState = false
        callbacks?.onError?.(e.error)
      }

      window.speechSynthesis.speak(utterance)
    } catch (err: unknown) {
      this.isSpeakingState = false
      callbacks?.onError?.(err instanceof Error ? err.message : "Speech playback error")
    }
  }

  public stop(): void {
    if (typeof window !== "undefined" && "speechSynthesis" in window) {
      try {
        window.speechSynthesis.cancel()
      } catch {
        // Ignore
      }
    }
    this.isSpeakingState = false
  }
}
