/**
 * Panoramic skyline illustration bridging heritage Indian bistro architecture
 * (India Gate, jharokha arches, tandoor chimneys) with modern green tech
 * (wind turbines, solar glass, rapid metro) and atmospheric nebula skies.
 * Styled dynamically for both Light and Dark modes.
 */
export function BistroPanorama() {
  return (
    <div className="relative w-full overflow-hidden border-y border-border/80 bg-gradient-to-b from-muted/30 via-background to-muted/20 select-none">
      {/* Sky & Nebula Ambient Backdrop */}
      <div className="absolute inset-0 bg-gradient-to-r from-primary/5 via-sky-500/5 to-amber-500/5 dark:from-primary/10 dark:via-indigo-950/20 dark:to-amber-500/10 pointer-events-none" />

      {/* Floating vector clouds (animated) */}
      <div className="absolute inset-0 pointer-events-none overflow-hidden" aria-hidden="true">
        {/* Cloud 1 */}
        <div className="absolute top-4 left-[8%] animate-float-slow opacity-60 dark:opacity-20 text-muted-foreground/30">
          <svg width="120" height="40" viewBox="0 0 120 40" fill="currentColor">
            <path d="M20 30h80a15 15 0 0 0 0-30 25 25 0 0 0-48-8 18 18 0 0 0-32 18 10 10 0 0 0 0 20z" />
          </svg>
        </div>
        {/* Cloud 2 */}
        <div className="absolute top-8 right-[15%] animate-float-reverse opacity-50 dark:opacity-15 text-muted-foreground/30">
          <svg width="140" height="45" viewBox="0 0 140 45" fill="currentColor">
            <path d="M25 35h90a18 18 0 0 0 0-36 28 28 0 0 0-54-9 20 20 0 0 0-36 20 12 12 0 0 0 0 25z" />
          </svg>
        </div>
        {/* Cloud 3 */}
        <div className="absolute top-12 left-[45%] animate-float-slow opacity-40 dark:opacity-10 text-muted-foreground/20">
          <svg width="90" height="30" viewBox="0 0 90 30" fill="currentColor">
            <path d="M15 25h60a12 12 0 0 0 0-24 20 20 0 0 0-38-6 15 15 0 0 0-22 15 8 8 0 0 0 0 15z" />
          </svg>
        </div>
      </div>

      {/* Panoramic SVG Scene */}
      <svg
        className="relative w-full h-40 sm:h-52 md:h-64 lg:h-72 object-cover block"
        viewBox="0 0 1600 320"
        preserveAspectRatio="xMidYMax slice"
        fill="none"
        xmlns="http://www.w3.org/2000/svg"
        aria-label="The Saffron Nebula Bistro and Delhi Skyline Panorama"
      >
        <defs>
          {/* Gradients */}
          <linearGradient id="skyGrad" x1="0" y1="0" x2="0" y2="1">
            <stop offset="0%" stopColor="var(--primary)" stopOpacity="0.04" />
            <stop offset="100%" stopColor="var(--background)" stopOpacity="0.9" />
          </linearGradient>

          <linearGradient id="monumentGrad" x1="0" y1="0" x2="0" y2="1">
            <stop offset="0%" stopColor="currentColor" stopOpacity="0.35" />
            <stop offset="100%" stopColor="currentColor" stopOpacity="0.12" />
          </linearGradient>

          <linearGradient id="bistroGrad" x1="0" y1="0" x2="0" y2="1">
            <stop offset="0%" stopColor="var(--primary)" stopOpacity="0.25" />
            <stop offset="100%" stopColor="var(--primary)" stopOpacity="0.08" />
          </linearGradient>

          <linearGradient id="riverGrad" x1="0" y1="0" x2="0" y2="1">
            <stop offset="0%" stopColor="#38bdf8" stopOpacity="0.22" />
            <stop offset="100%" stopColor="#0284c7" stopOpacity="0.08" />
          </linearGradient>
        </defs>

        {/* Deep Horizon Skyline: Far Buildings & Towers */}
        <g className="text-muted-foreground/30">
          {/* Far skyscrapers left */}
          <rect x="80" y="160" width="30" height="120" rx="2" fill="currentColor" opacity="0.4" />
          <rect x="120" y="140" width="45" height="140" rx="2" fill="currentColor" opacity="0.3" />
          <rect x="175" y="175" width="25" height="105" rx="2" fill="currentColor" opacity="0.5" />
          <line x1="142" y1="140" x2="142" y2="110" stroke="currentColor" strokeWidth="2" opacity="0.5" />

          {/* Wind Turbines (Left) */}
          <g transform="translate(60, 180)">
            <line x1="0" y1="0" x2="0" y2="100" stroke="currentColor" strokeWidth="2.5" />
            <circle cx="0" cy="0" r="3.5" fill="currentColor" />
            <g className="animate-spin-slow origin-center">
              <line x1="0" y1="0" x2="0" y2="-40" stroke="currentColor" strokeWidth="2" strokeLinecap="round" />
              <line x1="0" y1="0" x2="35" y2="20" stroke="currentColor" strokeWidth="2" strokeLinecap="round" />
              <line x1="0" y1="0" x2="-35" y2="20" stroke="currentColor" strokeWidth="2" strokeLinecap="round" />
            </g>
          </g>

          <g transform="translate(230, 200)">
            <line x1="0" y1="0" x2="0" y2="80" stroke="currentColor" strokeWidth="2" />
            <circle cx="0" cy="0" r="3" fill="currentColor" />
            <g className="animate-spin-slow origin-center" style={{ animationDuration: "16s" }}>
              <line x1="0" y1="0" x2="0" y2="-32" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" />
              <line x1="0" y1="0" x2="28" y2="16" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" />
              <line x1="0" y1="0" x2="-28" y2="16" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" />
            </g>
          </g>
        </g>

        {/* Central Landmarks: India Gate & Historic Minaret Silhouettes */}
        <g className="text-foreground/40">
          {/* India Gate Silhouette (x: 480 to 600) */}
          <g transform="translate(490, 140)">
            {/* Top Attic */}
            <path d="M10 20 L90 20 L85 0 L15 0 Z" fill="currentColor" opacity="0.6" />
            {/* Main Arch Block */}
            <rect x="0" y="20" width="100" height="120" fill="currentColor" opacity="0.45" />
            {/* Central Archway Cutout */}
            <path d="M30 140 L30 75 Q50 45 70 75 L70 140 Z" fill="var(--background)" />
            {/* Inscribed horizontal banding */}
            <line x1="0" y1="40" x2="100" y2="40" stroke="var(--background)" strokeWidth="2" />
            <line x1="5" y1="22" x2="95" y2="22" stroke="var(--background)" strokeWidth="1.5" />
          </g>

          {/* Qutub Minar Silhouette (x: 880) */}
          <g transform="translate(870, 70)">
            <path d="M18 190 L12 0 L24 0 L18 190 Z" fill="currentColor" opacity="0.55" />
            {/* Fluted Balconies */}
            <rect x="9" y="45" width="18" height="4" rx="1" fill="currentColor" opacity="0.9" />
            <rect x="11" y="95" width="14" height="4" rx="1" fill="currentColor" opacity="0.9" />
            <rect x="13" y="145" width="10" height="4" rx="1" fill="currentColor" opacity="0.9" />
            <circle cx="18" cy="-3" r="2" fill="currentColor" />
          </g>

          {/* Lotus Temple Arches Silhouette (x: 1000) */}
          <g transform="translate(980, 155)">
            {/* Petals */}
            <path d="M40 105 C25 65 30 20 50 5 C70 20 75 65 60 105 Z" fill="currentColor" opacity="0.6" />
            <path d="M20 105 C10 75 18 35 35 25 C30 50 35 85 40 105 Z" fill="currentColor" opacity="0.45" />
            <path d="M80 105 C90 75 82 35 65 25 C70 50 65 85 60 105 Z" fill="currentColor" opacity="0.45" />
          </g>
        </g>

        {/* The Saffron Nebula Bistro Architecture (Center-Right & Foreground) */}
        <g>
          {/* Bistro Pavilion Terrace (x: 640 to 820) */}
          <g transform="translate(640, 160)">
            {/* Terracotta Canopy / Jharokha Roof */}
            <path d="M10 35 Q90 -10 170 35 L160 55 L20 55 Z" fill="url(#bistroGrad)" />
            <path d="M20 55 L160 55 L155 120 L25 120 Z" fill="currentColor" className="text-card" opacity="0.9" stroke="var(--border)" />
            {/* Jharokha Arched Windows */}
            <path d="M45 110 L45 80 Q60 65 75 80 L75 110 Z" fill="var(--primary)" opacity="0.3" />
            <path d="M105 110 L105 80 Q120 65 135 80 L135 110 Z" fill="var(--primary)" opacity="0.3" />
            {/* Bistro Signboard */}
            <rect x="50" y="38" width="80" height="14" rx="3" fill="var(--primary)" />
            <text x="90" y="48" textAnchor="middle" fill="#ffffff" fontSize="8" fontWeight="bold" letterSpacing="0.5">
              THE SAFFRON NEBULA
            </text>
            {/* Tandoor Chimney with gentle steam pulse */}
            <rect x="145" y="15" width="12" height="40" fill="currentColor" className="text-muted-foreground" opacity="0.7" />
            <circle cx="151" cy="6" r="4" fill="var(--primary)" className="animate-pulse" opacity="0.6" />
            <circle cx="153" cy="-4" r="6" fill="var(--primary)" className="animate-pulse" opacity="0.4" />
          </g>

          {/* Outdoor Heated Dining Patio (Tables & Lanterns) */}
          <g transform="translate(280, 220)">
            {/* Table 1 */}
            <rect x="20" y="35" width="40" height="6" rx="2" fill="var(--primary)" opacity="0.8" />
            <rect x="37" y="41" width="6" height="25" fill="currentColor" className="text-muted-foreground" />
            {/* Chairs */}
            <path d="M10 40 L16 40 L16 66 L10 66 Z" fill="currentColor" className="text-muted-foreground/60" />
            <path d="M64 40 L70 40 L70 66 L64 66 Z" fill="currentColor" className="text-muted-foreground/60" />
            {/* Table Candle */}
            <circle cx="40" cy="30" r="3" fill="#f59e0b" className="animate-pulse" />
          </g>

          <g transform="translate(390, 225)">
            {/* Table 2 */}
            <rect x="20" y="35" width="40" height="6" rx="2" fill="var(--primary)" opacity="0.8" />
            <rect x="37" y="41" width="6" height="20" fill="currentColor" className="text-muted-foreground" />
            {/* Table Candle */}
            <circle cx="40" cy="30" r="3" fill="#f59e0b" className="animate-pulse" />
          </g>

          {/* Fairy-Lit Pergola String Lights */}
          <path d="M260 210 Q360 235 460 210 Q560 235 660 210" stroke="var(--primary)" strokeWidth="1.5" strokeDasharray="3 3" fill="none" opacity="0.6" />
          <circle cx="310" cy="222" r="2.5" fill="#f59e0b" />
          <circle cx="360" cy="224" r="2.5" fill="#f59e0b" />
          <circle cx="410" cy="222" r="2.5" fill="#f59e0b" />
          <circle cx="510" cy="222" r="2.5" fill="#f59e0b" />
          <circle cx="560" cy="224" r="2.5" fill="#f59e0b" />
          <circle cx="610" cy="222" r="2.5" fill="#f59e0b" />
        </g>

        {/* Rapid Metro Viaduct & Train (x: 1120 to 1550) */}
        <g transform="translate(1120, 200)">
          {/* Pillars */}
          <rect x="40" y="30" width="12" height="60" fill="currentColor" className="text-muted-foreground/40" />
          <rect x="140" y="30" width="12" height="60" fill="currentColor" className="text-muted-foreground/40" />
          <rect x="240" y="30" width="12" height="60" fill="currentColor" className="text-muted-foreground/40" />
          <rect x="340" y="30" width="12" height="60" fill="currentColor" className="text-muted-foreground/40" />
          {/* Deck */}
          <rect x="0" y="22" width="450" height="8" rx="2" fill="currentColor" className="text-muted-foreground/60" />
          {/* Sleek Train */}
          <rect x="80" y="6" width="180" height="16" rx="4" fill="var(--primary)" opacity="0.9" />
          {/* Windows */}
          <rect x="90" y="10" width="16" height="7" rx="1" fill="#ffffff" opacity="0.8" />
          <rect x="112" y="10" width="16" height="7" rx="1" fill="#ffffff" opacity="0.8" />
          <rect x="134" y="10" width="16" height="7" rx="1" fill="#ffffff" opacity="0.8" />
          <rect x="156" y="10" width="16" height="7" rx="1" fill="#ffffff" opacity="0.8" />
          <rect x="178" y="10" width="16" height="7" rx="1" fill="#ffffff" opacity="0.8" />
          <rect x="200" y="10" width="16" height="7" rx="1" fill="#ffffff" opacity="0.8" />
          <rect x="222" y="10" width="16" height="7" rx="1" fill="#ffffff" opacity="0.8" />
        </g>

        {/* Riverfront Promenade & Water Waves (Bottom) */}
        <path
          d="M0 280 Q200 270 400 285 T800 275 T1200 285 T1600 275 L1600 320 L0 320 Z"
          fill="url(#riverGrad)"
        />
        <path
          d="M0 295 Q300 285 600 298 T1200 290 T1600 298"
          stroke="#38bdf8"
          strokeWidth="1.5"
          strokeOpacity="0.3"
          fill="none"
        />

        {/* Floating audio soundwave ribbon across the horizon */}
        <g transform="translate(700, 290)" opacity="0.7">
          <rect x="0" y="-8" width="3" height="16" rx="1.5" fill="var(--primary)" />
          <rect x="8" y="-14" width="3" height="28" rx="1.5" fill="var(--primary)" />
          <rect x="16" y="-22" width="3" height="44" rx="1.5" fill="var(--primary)" />
          <rect x="24" y="-12" width="3" height="24" rx="1.5" fill="var(--primary)" />
          <rect x="32" y="-18" width="3" height="36" rx="1.5" fill="var(--primary)" />
          <rect x="40" y="-6" width="3" height="12" rx="1.5" fill="var(--primary)" />
          <rect x="48" y="-15" width="3" height="30" rx="1.5" fill="var(--primary)" />
          <rect x="56" y="-10" width="3" height="20" rx="1.5" fill="var(--primary)" />
        </g>
      </svg>

      {/* Panorama Caption & Coordinate Badge */}
      <div className="max-w-6xl mx-auto px-4 sm:px-6 lg:px-8 py-2.5 flex flex-wrap items-center justify-between text-[11px] font-mono text-muted-foreground border-t border-border/50">
        <div className="flex items-center gap-2">
          <span className="size-2 rounded-full bg-emerald-500 animate-pulse" />
          <span className="font-semibold text-foreground">The Saffron Nebula Bistro</span>
          <span>·</span>
          <span>Connaught Outer Circle & Outdoor Courtyard</span>
        </div>
        <div className="flex items-center gap-4">
          <span>LAT 28.6328° N, 77.2197° E</span>
          <span className="hidden sm:inline text-border">|</span>
          <span className="hidden sm:inline text-primary font-medium">14 Tables Live on MCP</span>
        </div>
      </div>
    </div>
  )
}
