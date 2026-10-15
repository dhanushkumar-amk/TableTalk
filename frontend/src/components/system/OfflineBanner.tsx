import { useState, useEffect } from "react"
import { WifiOff, Wifi } from "lucide-react"

/**
 * Clean, subtle global banner that notifies the user when their internet connection drops
 * and confirms reconnection when restored.
 */
export function OfflineBanner() {
  const [isOffline, setIsOffline] = useState(!navigator.onLine)
  const [showReconnected, setShowReconnected] = useState(false)

  useEffect(() => {
    const handleOnline = () => {
      setIsOffline(false)
      setShowReconnected(true)
      const timer = setTimeout(() => setShowReconnected(false), 3000)
      return () => clearTimeout(timer)
    }

    const handleOffline = () => {
      setIsOffline(true)
      setShowReconnected(false)
    }

    window.addEventListener("online", handleOnline)
    window.addEventListener("offline", handleOffline)

    return () => {
      window.removeEventListener("online", handleOnline)
      window.removeEventListener("offline", handleOffline)
    }
  }, [])

  if (isOffline) {
    return (
      <div
        role="status"
        aria-live="assertive"
        className="sticky top-0 z-50 flex items-center justify-center gap-2 bg-amber-600 px-4 py-2 text-xs font-medium text-white shadow-sm"
      >
        <WifiOff className="size-4 animate-pulse" />
        <span>You are currently offline. Changes will sync when your connection is restored.</span>
      </div>
    )
  }

  if (showReconnected) {
    return (
      <div
        role="status"
        aria-live="polite"
        className="sticky top-0 z-50 flex items-center justify-center gap-2 bg-emerald-600 px-4 py-2 text-xs font-medium text-white shadow-sm"
      >
        <Wifi className="size-4" />
        <span>Connection restored. TableTalk is back online.</span>
      </div>
    )
  }

  return null
}
