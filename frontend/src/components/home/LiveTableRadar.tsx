import { useState } from "react"
import { motion, useReducedMotion } from "framer-motion"
import { MapPin, Users, CheckCircle2, Clock } from "lucide-react"

interface TableNode {
  id: string
  name: string
  area: "Patio Garden" | "Main Dining" | "Private Booths"
  capacity: number
  status: "available" | "reserved" | "occupied"
  x: number
  y: number
  eta?: string
}

const TABLES: TableNode[] = [
  // Patio Garden (Tables 1-4)
  { id: "T-01", name: "Patio 01", area: "Patio Garden", capacity: 2, status: "available", x: 18, y: 28 },
  { id: "T-02", name: "Patio 02", area: "Patio Garden", capacity: 4, status: "occupied", eta: "15m", x: 38, y: 22 },
  { id: "T-03", name: "Patio 03", area: "Patio Garden", capacity: 4, status: "reserved", x: 18, y: 55 },
  { id: "T-04", name: "Patio 04", area: "Patio Garden", capacity: 4, status: "available", x: 38, y: 55 },

  // Main Dining Hall (Tables 5-10)
  { id: "T-05", name: "Dining 05", area: "Main Dining", capacity: 2, status: "occupied", eta: "25m", x: 60, y: 24 },
  { id: "T-06", name: "Dining 06", area: "Main Dining", capacity: 4, status: "available", x: 80, y: 24 },
  { id: "T-07", name: "Dining 07", area: "Main Dining", capacity: 4, status: "occupied", eta: "10m", x: 60, y: 52 },
  { id: "T-08", name: "Dining 08", area: "Main Dining", capacity: 6, status: "reserved", x: 80, y: 52 },
  { id: "T-09", name: "Dining 09", area: "Main Dining", capacity: 2, status: "available", x: 60, y: 80 },
  { id: "T-10", name: "Dining 10", area: "Main Dining", capacity: 4, status: "occupied", eta: "35m", x: 80, y: 80 },

  // Private Booths (Tables 11-14)
  { id: "T-11", name: "Booth 11", area: "Private Booths", capacity: 6, status: "reserved", x: 18, y: 82 },
  { id: "T-12", name: "Booth 12", area: "Private Booths", capacity: 8, status: "occupied", eta: "45m", x: 38, y: 82 },
  { id: "T-13", name: "Booth 13", area: "Private Booths", capacity: 6, status: "available", x: 92, y: 35 },
  { id: "T-14", name: "Booth 14", area: "Private Booths", capacity: 8, status: "occupied", eta: "20m", x: 92, y: 68 },
]

export function LiveTableRadar() {
  const shouldReduceMotion = useReducedMotion()
  const [selectedTable, setSelectedTable] = useState<TableNode>(TABLES[3]) // Default to Patio 04

  const availableCount = TABLES.filter((t) => t.status === "available").length

  return (
    <div className="rounded-2xl border border-border/80 bg-card p-5 sm:p-6 shadow-dropdown space-y-4">
      {/* Header Bar */}
      <div className="flex flex-wrap items-center justify-between gap-3 pb-3 border-b border-border/60">
        <div className="space-y-0.5">
          <div className="text-[11px] font-mono uppercase tracking-wider text-primary font-semibold flex items-center gap-1.5">
            <span className="relative flex size-2">
              <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-emerald-400 opacity-75" />
              <span className="relative inline-flex rounded-full size-2 bg-emerald-500" />
            </span>
            <span>LIVE RESTAURANT FLOORPLAN RADAR</span>
          </div>
          <h3 className="text-base font-semibold text-foreground tracking-tight">
            14 Interactive Dining Tables
          </h3>
        </div>

        {/* Real-time Status Chip */}
        <div className="flex items-center gap-2 px-3 py-1 rounded-full bg-muted/60 border border-border/70 text-xs font-mono">
          <span className="text-emerald-500 font-bold">{availableCount} Available</span>
          <span className="text-muted-foreground">/ 14 Total</span>
        </div>
      </div>

      {/* SVG Floorplan Canvas Grid */}
      <div className="relative h-64 sm:h-72 w-full rounded-xl border border-border/60 bg-muted/20 overflow-hidden p-3 select-none">
        {/* Floorplan Zone Watermark Lines */}
        <div className="absolute inset-0 flex">
          <div className="w-[50%] border-r border-dashed border-border/60 p-2 text-[10px] font-mono text-muted-foreground/60 uppercase">
            Heated Patio Garden
          </div>
          <div className="w-[50%] p-2 text-[10px] font-mono text-muted-foreground/60 uppercase">
            Main Dining Room & Booths
          </div>
        </div>

        {/* Render 14 Tables */}
        {TABLES.map((t) => {
          const isSelected = selectedTable.id === t.id
          let statusColor = "bg-emerald-500 border-emerald-400"
          let ringColor = "ring-emerald-500/30"
          if (t.status === "occupied") {
            statusColor = "bg-amber-500 border-amber-400"
            ringColor = "ring-amber-500/30"
          } else if (t.status === "reserved") {
            statusColor = "bg-primary border-primary"
            ringColor = "ring-primary/30"
          }

          return (
            <motion.button
              key={t.id}
              onClick={() => setSelectedTable(t)}
              whileHover={shouldReduceMotion ? {} : { scale: 1.15 }}
              whileTap={shouldReduceMotion ? {} : { scale: 0.95 }}
              className={`absolute -translate-x-1/2 -translate-y-1/2 rounded-lg p-1.5 transition-all cursor-pointer flex flex-col items-center justify-center ${
                isSelected
                  ? "bg-background border-2 border-primary shadow-lg ring-4 " + ringColor + " z-20"
                  : "bg-background/80 border border-border/80 hover:border-foreground/40 shadow-xs z-10"
              }`}
              style={{ left: `${t.x}%`, top: `${t.y}%` }}
              aria-label={`${t.name} - ${t.status}`}
            >
              <div className="flex items-center gap-1">
                <span className={`size-2 rounded-full ${statusColor}`} />
                <span className="text-[10px] font-mono font-bold text-foreground">
                  {t.id.replace("T-", "#")}
                </span>
              </div>
              <span className="text-[9px] text-muted-foreground font-mono">
                {t.capacity}p
              </span>
            </motion.button>
          )
        })}
      </div>

      {/* Selected Table Inspection Card */}
      <div className="rounded-xl border border-border/80 bg-background/90 p-3.5 flex flex-wrap items-center justify-between gap-3 text-xs">
        <div className="flex items-center gap-3">
          <div className="flex size-9 items-center justify-center rounded-lg bg-primary/10 text-primary font-mono font-bold text-sm">
            {selectedTable.id}
          </div>
          <div>
            <div className="flex items-center gap-2">
              <span className="font-semibold text-foreground">{selectedTable.name}</span>
              <span className="text-muted-foreground">·</span>
              <span className="text-muted-foreground flex items-center gap-1">
                <MapPin className="size-3 text-primary" />
                {selectedTable.area}
              </span>
            </div>
            <div className="text-[11px] text-muted-foreground flex items-center gap-2 mt-0.5">
              <span className="flex items-center gap-1">
                <Users className="size-3" />
                Capacity: {selectedTable.capacity} Guests
              </span>
              {selectedTable.eta && (
                <span className="flex items-center gap-1 text-amber-600 dark:text-amber-400">
                  <Clock className="size-3" />
                  Est. Open in {selectedTable.eta}
                </span>
              )}
            </div>
          </div>
        </div>

        {/* Availability Badge */}
        <div>
          {selectedTable.status === "available" && (
            <span className="inline-flex items-center gap-1 px-2.5 py-1 rounded-md bg-emerald-500/10 text-emerald-700 dark:text-emerald-300 font-mono text-[11px] font-semibold border border-emerald-500/20">
              <CheckCircle2 className="size-3" />
              Direct AI Lock Available
            </span>
          )}
          {selectedTable.status === "reserved" && (
            <span className="inline-flex items-center gap-1 px-2.5 py-1 rounded-md bg-primary/10 text-primary font-mono text-[11px] font-semibold border border-primary/20">
              Reserved for 7:30 PM
            </span>
          )}
          {selectedTable.status === "occupied" && (
            <span className="inline-flex items-center gap-1 px-2.5 py-1 rounded-md bg-amber-500/10 text-amber-700 dark:text-amber-400 font-mono text-[11px] font-semibold border border-amber-500/20">
              Dining in Progress
            </span>
          )}
        </div>
      </div>

      {/* Legend */}
      <div className="flex items-center justify-between text-[11px] font-mono text-muted-foreground pt-1 border-t border-border/50">
        <div className="flex items-center gap-4">
          <span className="flex items-center gap-1.5">
            <span className="size-2 rounded-full bg-emerald-500" />
            Available Now
          </span>
          <span className="flex items-center gap-1.5">
            <span className="size-2 rounded-full bg-primary" />
            AI Locked
          </span>
          <span className="flex items-center gap-1.5">
            <span className="size-2 rounded-full bg-amber-500" />
            Occupied
          </span>
        </div>
        <span className="text-[10px] hidden sm:inline">Click table to inspect</span>
      </div>
    </div>
  )
}
