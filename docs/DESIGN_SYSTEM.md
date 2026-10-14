# TableTalk Design System

## 1. Design Direction & Philosophy

TableTalk adopts a startup-grade design discipline characterized by **restraint, generous whitespace, disciplined typography, and tactile precision** (drawing aesthetic inspiration from products like Linear, Vercel, Stripe, and Cal.com).

### Core Principles
1. **Calm Minimalism**: High whitespace ratios, quiet surfaces, and high legibility.
2. **Singular Focal Accent**: One distinct culinary accent color rather than multi-color visual noise.
3. **Hairline Precision**: 1px low-contrast borders instead of heavy drop shadows. Layered soft elevation is reserved exclusively for floating overlays (command palette, dropdowns, sheets).
4. **Safety-Critical Discipline**: Never imply allergen safety (no "safe", "allergy-free", or green checks for allergens). Real-time payment deposit flows are explicitly branded with a calm `SimulatedBanner`.
5. **Accessibility-First**: WCAG 2.2 AA compliant across light and dark modes, visible focus rings on interactive elements, 44px+ touch targets, and full keyboard navigation.

---

## 2. Color Palette & Token Architecture

All design tokens are defined as standard CSS variables in `frontend/src/index.css` using the modern OKLCH color space for perceptual uniformity.

### 2.1 Accent Selection: Deep Terracotta
* **Light Accent (`--primary`)**: `oklch(0.56 0.165 38)`
* **Dark Accent (`--primary`)**: `oklch(0.68 0.155 38)`
* **Primary Foreground**: `oklch(0.99 0.004 65)` (light) / `oklch(0.13 0.006 65)` (dark)
* **Accent Background Tint**: `oklch(0.95 0.025 38)` (light) / `oklch(0.25 0.04 38)` (dark)

> **Justification**: TableTalk serves "The Saffron Nebula Bistro", a contemporary Indian culinary experience. The warm **Deep Terracotta** palette honors the earthy clay tandoor, terracotta pottery, and aromatic roasted spices (saffron, cardamom, and Kashmiri chili). Unlike clinical tech blues or emerald fintech tones, terracotta introduces culinary warmth, appetite appeal, and refined luxury while exceeding WCAG AA contrast standards (>4.5:1).

### 2.2 Neutral Scales (Warm Stone / Zinc)
| Token | Light Value | Dark Value | Purpose |
|:---|:---|:---|:---|
| `--background` | `oklch(0.985 0.003 75)` | `oklch(0.14 0.006 65)` | Base viewport surface |
| `--foreground` | `oklch(0.18 0.01 65)` | `oklch(0.97 0.004 65)` | Primary high-contrast text |
| `--card` | `oklch(1 0 0)` | `oklch(0.18 0.006 65)` | Elevated content cards |
| `--card-foreground` | `oklch(0.18 0.01 65)` | `oklch(0.97 0.004 65)` | Text on card surfaces |
| `--muted` | `oklch(0.955 0.006 75)` | `oklch(0.22 0.006 65)` | Subdued backgrounds |
| `--muted-foreground` | `oklch(0.48 0.015 65)` | `oklch(0.68 0.01 65)` | Secondary labels & hints |
| `--border` | `oklch(0.91 0.006 75)` | `oklch(1 0 0 / 10%)` | 1px hairline card borders |
| `--input` | `oklch(0.91 0.006 75)` | `oklch(1 0 0 / 14%)` | Input control borders |
| `--ring` | `oklch(0.56 0.165 38 / 35%)`| `oklch(0.68 0.155 38 / 45%)`| Visible focus rings |

### 2.3 Semantic Tokens
* **Success**: `oklch(0.58 0.16 148)` (Light) / `oklch(0.70 0.15 148)` (Dark)
* **Warning**: `oklch(0.72 0.16 75)` (Light) / `oklch(0.80 0.15 75)` (Dark)
* **Destructive**: `oklch(0.58 0.22 25)` (Light) / `oklch(0.68 0.19 25)` (Dark)
* **Rule**: In adherence with allergen safety standards, semantic green checks or "success" badges are strictly forbidden on allergen lists.

---

## 3. Typography

All typography is self-hosted via Fontsource npm packages (`@fontsource-variable/geist` and `@fontsource-variable/geist-mono`). **Zero CDNs, zero remote font calls, and zero external network requests** are made at runtime.

| Level | Size / Weight | Tracking | Usage |
|:---|:---|:---|:---|
| **Display** | `2.5rem` (40px) / `600 SemiBold` | `-0.025em` | Hero landing statements |
| **H1** | `1.875rem` (30px) / `600 SemiBold` | `-0.02em` | Page titles (`PageHeader`) |
| **H2** | `1.25rem` (20px) / `600 SemiBold` | `-0.015em` | Section headers (`Section`) |
| **H3** | `1rem` (16px) / `500 Medium` | `-0.01em` | Card titles and subheaders |
| **Body** | `0.875rem` (14px) / `400 Regular` | `0` | Default reading copy (1.6 line height) |
| **Small** | `0.75rem` (12px) / `400 Regular` | `0` | Metadata, timestamps, helper text |
| **Mono** | `0.75rem` - `0.875rem` / `600 SemiBold` | `+0.05em` | Confirmation codes, order codes, tokens |

---

## 4. Radii, Hairlines & Elevation

* **Control Radius**: `0.625rem` (10px) — buttons, inputs, segmented controls.
* **Card Radius**: `0.875rem` (14px) to `1rem` (16px) — content surfaces and dialogs.
* **Badge Radius**: `9999px` — pills, status chips, countdown chips.
* **Elevation**:
  - Cards use 1px hairline borders (`border border-border/80`) with `shadow-xs`.
  - Floating overlays (Command Palette, Dropdowns) use soft layered drop shadows (`shadow-dropdown` and `shadow-2xl`).

---

## 5. Motion Guidelines

* **Transitions**: 150ms – 220ms ease-out using cubic-bezier timing `cubic-bezier(0.16, 1, 0.3, 1)`.
* **Nature**: Subtle fades and 4px micro-slides. No decorative bounces or jarring animations.
* **Accessibility**: Respects `prefers-reduced-motion: reduce` across Framer Motion / Motion primitives with instant fallbacks.

---

## 6. Primitives Library (`src/components/primitives`)

1. **`PageHeader`**: Standardized page title, category eyebrow, optional status badge, and action button slot.
2. **`Section`**: Structured container with optional title, subtitle description, and action button.
3. **`EmptyState`**: Centered icon container, title, description, and primary/secondary call-to-actions.
4. **`Skeleton`**: Accessible pulse loading skeletons with dedicated variants: `CardSkeleton`, `TextSkeleton`, `ButtonSkeleton`, and `ListItemSkeleton`.
5. **`StatusBadge`**: Multi-variant status badge (`neutral`, `success`, `warning`, `danger`, `accent`) combining an icon and explicit text so status is never conveyed by color alone.
6. **`Money`**: Formatted currency display (e.g., `₹1,280`) featuring tabular numerals and accessible `aria-label` screen reader announcements.
7. **`CodeBlock`**: High-contrast monospace code display with 1-click clipboard copy button and toast feedback.
8. **`CountdownChip`**: Live-ticking tabular countdown badge that transitions to an animated warning state when <5 minutes remain. Never extends server deadlines.
9. **`SimulatedBanner`**: Distinctive, calm banner informing users that payments and booking holds are demonstration previews without real money movement.

---

## 7. App Shell Architecture

1. **Desktop TopNav**:
   - Wordmark "TableTalk" text with terracotta accent indicator (no third-party or corporate logos).
   - Navigation links: **Assistant**, **Menu**, **Reserve**, **Order**, **Manage**.
   - Search & Command Palette trigger button (`⌘K` / `Ctrl+K`).
   - Theme toggle button (Light, Dark, System) with local storage persistence and zero flash on load.
   - Privacy link.
2. **Mobile BottomTabBar**:
   - 5 core destinations with 20px Lucide icons and labels.
   - Safe-area bottom padding (`pb-safe`) for edge-to-edge mobile viewports.
   - 44px minimum touch targets.
3. **Command Palette (`cmdk`)**:
   - Instant page jumps (`/assistant`, `/menu`, `/reserve`, `/order`, `/manage`, `/privacy`, `/dev/cards`).
   - In-palette theme switching.
4. **Error Boundary & 404 Route**:
   - Production error boundary catching unhandled rendering errors with safe reload and return actions.
   - Clean 404 page directing visitors back to the voice assistant.
