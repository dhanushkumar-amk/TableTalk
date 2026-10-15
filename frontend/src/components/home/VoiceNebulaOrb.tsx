import { motion, useReducedMotion } from "framer-motion"

interface VoiceNebulaOrbProps {
  isSpeaking?: boolean
}

/**
 * Animated SVG Voice Nebula Orb:
 * Represents the AI voice engine nucleus with glowing concentric acoustic ripples,
 * orbital energy rings, and subtle floating particles.
 */
export function VoiceNebulaOrb({ isSpeaking = true }: VoiceNebulaOrbProps) {
  const shouldReduceMotion = useReducedMotion()

  return (
    <div className="relative flex items-center justify-center size-44 sm:size-52 mx-auto select-none pointer-events-none">
      {/* Outer ambient blur glow */}
      <div className="absolute inset-0 rounded-full bg-gradient-to-tr from-primary/30 via-amber-500/20 to-primary/40 blur-2xl opacity-70 animate-pulse" />

      {/* SVG Voice Acoustic Ripple Rings */}
      <svg
        viewBox="0 0 200 200"
        className="relative size-full overflow-visible"
        fill="none"
        xmlns="http://www.w3.org/2000/svg"
        aria-hidden="true"
      >
        <defs>
          <radialGradient id="orbCore" cx="50%" cy="50%" r="50%">
            <stop offset="0%" stopColor="var(--primary)" stopOpacity="0.9" />
            <stop offset="60%" stopColor="#f59e0b" stopOpacity="0.6" />
            <stop offset="100%" stopColor="var(--primary)" stopOpacity="0.1" />
          </radialGradient>

          <linearGradient id="ringGrad" x1="0" y1="0" x2="1" y2="1">
            <stop offset="0%" stopColor="var(--primary)" stopOpacity="0.8" />
            <stop offset="50%" stopColor="#38bdf8" stopOpacity="0.5" />
            <stop offset="100%" stopColor="var(--primary)" stopOpacity="0.8" />
          </linearGradient>
        </defs>

        {/* Acoustic Pulse Wave 1 */}
        <motion.circle
          cx="100"
          cy="100"
          r="78"
          stroke="url(#ringGrad)"
          strokeWidth="1.5"
          strokeDasharray="4 6"
          opacity={0.4}
          animate={
            shouldReduceMotion
              ? {}
              : {
                  scale: isSpeaking ? [1, 1.15, 1] : 1,
                  rotate: [0, 180, 360],
                  opacity: [0.3, 0.6, 0.3],
                }
          }
          transition={{ duration: 12, repeat: Infinity, ease: "linear" }}
          style={{ transformOrigin: "center" }}
        />

        {/* Acoustic Pulse Wave 2 */}
        <motion.circle
          cx="100"
          cy="100"
          r="62"
          stroke="var(--primary)"
          strokeWidth="1.5"
          strokeDasharray="6 4"
          opacity={0.5}
          animate={
            shouldReduceMotion
              ? {}
              : {
                  scale: isSpeaking ? [1, 1.08, 1] : 1,
                  rotate: [360, 180, 0],
                  opacity: [0.4, 0.7, 0.4],
                }
          }
          transition={{ duration: 8, repeat: Infinity, ease: "linear" }}
          style={{ transformOrigin: "center" }}
        />

        {/* Glowing Energy Core */}
        <circle cx="100" cy="100" r="44" fill="url(#orbCore)" />

        {/* Inner Waveform Ribs */}
        <motion.g
          animate={
            shouldReduceMotion
              ? {}
              : {
                  scale: isSpeaking ? [0.95, 1.05, 0.95] : 1,
                }
          }
          transition={{ duration: 2, repeat: Infinity, ease: "easeInOut" }}
          style={{ transformOrigin: "center" }}
        >
          {/* Audio Bars inside core */}
          <line x1="76" y1="92" x2="76" y2="108" stroke="#ffffff" strokeWidth="2.5" strokeLinecap="round" opacity={0.8} />
          <line x1="84" y1="84" x2="84" y2="116" stroke="#ffffff" strokeWidth="2.5" strokeLinecap="round" opacity={0.9} />
          <line x1="92" y1="76" x2="92" y2="124" stroke="#ffffff" strokeWidth="2.5" strokeLinecap="round" opacity={1} />
          <line x1="100" y1="70" x2="100" y2="130" stroke="#ffffff" strokeWidth="3" strokeLinecap="round" opacity={1} />
          <line x1="108" y1="76" x2="108" y2="124" stroke="#ffffff" strokeWidth="2.5" strokeLinecap="round" opacity={1} />
          <line x1="116" y1="84" x2="116" y2="116" stroke="#ffffff" strokeWidth="2.5" strokeLinecap="round" opacity={0.9} />
          <line x1="124" y1="92" x2="124" y2="108" stroke="#ffffff" strokeWidth="2.5" strokeLinecap="round" opacity={0.8} />
        </motion.g>

        {/* Floating Orbital Satellite Dots */}
        <motion.circle
          cx="100"
          cy="28"
          r="3"
          fill="#38bdf8"
          animate={shouldReduceMotion ? {} : { rotate: 360 }}
          transition={{ duration: 6, repeat: Infinity, ease: "linear" }}
          style={{ transformOrigin: "100px 100px" }}
        />
        <motion.circle
          cx="100"
          cy="168"
          r="2.5"
          fill="#f59e0b"
          animate={shouldReduceMotion ? {} : { rotate: -360 }}
          transition={{ duration: 9, repeat: Infinity, ease: "linear" }}
          style={{ transformOrigin: "100px 100px" }}
        />
      </svg>
    </div>
  )
}
