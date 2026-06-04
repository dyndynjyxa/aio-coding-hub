# Style Guidelines

> Frontend style layering, design tokens, and drift checks for the React/Tauri UI.

---

## Style Layers

Use these layers from lowest level to highest level:

1. **CSS variables** — shadcn-standard semantic tokens in `src/styles/globals.css`
   (`:root` for light, `.dark` for dark; HSL values without wrapper).
2. **Tailwind bridge** — `tailwind.config.ts` maps CSS variables to Tailwind
   utilities (`bg-background`, `text-foreground`, `border-border`, etc.) and
   registers brand/status/chart colors.
3. **Programmatic colors** — `src/constants/colors.ts` (`BRAND`, `STATUS`,
   `CHART_PALETTE`); chart-specific Recharts adapters in
   `src/components/charts/chartTheme.ts`.
4. **UI primitives** — shadcn components in `src/ui/shadcn/*`, wrapped by public
   API components in `src/ui/*`.
5. **Feature code** — pages, components, and hooks compose public UI primitives
   plus Tailwind utility classes.

Do not add page-local color systems, one-off shadow scales, or local shadcn
wrappers in feature folders.

---

## Semantic Token System

The project uses the **shadcn/ui standard CSS variable convention** (Tailwind v3,
HSL format without wrapper).

### Color Tokens

| Token | Light | Dark | Tailwind class |
|-------|-------|------|----------------|
| `--background` / `--foreground` | Page surface / text | Inverted | `bg-background`, `text-foreground` |
| `--card` / `--card-foreground` | Card surface / text | slate-800 region | `bg-card`, `text-card-foreground` |
| `--popover` / `--popover-foreground` | Overlay surface / text | — | `bg-popover`, `text-popover-foreground` |
| `--primary` / `--primary-foreground` | Premium indigo `#5153EC` / white | Same | `bg-primary`, `text-primary-foreground` |
| `--secondary` / `--secondary-foreground` | Subtle surface / text | — | `bg-secondary`, `text-secondary-foreground` |
| `--muted` / `--muted-foreground` | Disabled/subtle surface / text | — | `bg-muted`, `text-muted-foreground` |
| `--accent` / `--accent-foreground` | Brand blue (= primary) / white | Same | `bg-accent`, `text-accent-foreground` |
| `--destructive` / `--destructive-foreground` | Error red / white | — | `bg-destructive`, `text-destructive-foreground` |
| `--border` | Default borders | — | `border-border` |
| `--input` | Form input borders | — | `border-input` |
| `--ring` | Focus ring (brand blue) | — | `ring-ring` |

### Brand & Status Colors (non-token, hex)

| Name | Value | Usage |
|------|-------|-------|
| `brand` / `accent` | `#5153EC` | Premium indigo primary (also exposed via `--primary` / `--accent`) |
| `accent-secondary` | `#6366F1` | Secondary indigo accent / gradient endpoint |
| `success` | `#34D399` | Success states |
| `warning` | `#FB923C` | Warning states |
| `danger` | `#F87171` | Destructive/error states |
| `info` | `#0EA5E9` | Informational states |

### Sidebar Tokens

Sidebar uses its own token set (`--sidebar-*`) to allow independent theming:
`sidebar`, `sidebar-foreground`, `sidebar-primary`, `sidebar-primary-foreground`,
`sidebar-accent`, `sidebar-accent-foreground`, `sidebar-border`, `sidebar-ring`.

### Chart Tokens

Chart colors use `--chart-1` through `--chart-5`, bridged as `chart-1` through
`chart-5` in Tailwind.

### Radius

`--radius: 0.625rem` is the premium base token. Derived values:
- `rounded-lg` = `var(--radius)` (10px)
- `rounded-md` = `calc(var(--radius) - 2px)` (8px)
- `rounded-sm` = `calc(var(--radius) - 4px)` (6px)

---

## Token Usage Rules

**New code must use semantic tokens, not hardcoded slate-\* classes.**

The canonical color path is:

1. define or reuse a semantic CSS variable in `src/styles/globals.css`;
2. expose it through `tailwind.config.ts`;
3. consume it as a semantic Tailwind utility such as `bg-card`,
   `text-muted-foreground`, `border-border`, or `ring-ring`.

Do not add new colors directly in page, feature, or route code. If a color is
needed by TypeScript, use `BRAND`, `STATUS`, or `CHART_PALETTE`; if a chart needs
library-specific fill/stroke values, keep the adapter in
`src/components/charts/chartTheme.ts`.

| Instead of | Use |
|------------|-----|
| `bg-white dark:bg-slate-800` | `bg-card` |
| `bg-slate-50 dark:bg-slate-950` | `bg-background` |
| `bg-slate-100 dark:bg-slate-800` | `bg-muted` or `bg-secondary` |
| `text-slate-900 dark:text-slate-100` | `text-foreground` |
| `text-slate-500 dark:text-slate-400` | `text-muted-foreground` |
| `border-slate-200 dark:border-slate-700` | `border-border` |
| `ring-accent/30 ring-offset-white dark:ring-offset-slate-900` | `ring-ring/30 ring-offset-background` |
| `hover:bg-slate-100 dark:hover:bg-slate-800` | `hover:bg-secondary` |

When using semantic tokens, the `dark:` prefix is unnecessary for colors — the
CSS variable swap handles dark mode automatically.

Status colors (emerald, amber, rose) still require explicit `dark:` variants
because they are not token-driven.

Use `BRAND`, `STATUS`, and `CHART_PALETTE` from `src/constants/colors.ts` when
TypeScript needs a programmatic color value.

Keep raw hex values out of TS/TSX feature code.

### Allowed raw color exceptions

- `src/constants/colors.ts`
- `src/components/charts/chartTheme.ts`
- CSS files and CSS variable definitions
- static assets under `src/assets/`
- tests and fixtures

Everything else in ordinary TS/TSX feature code is treated as style drift, even
when the audit only reports it. Prefer moving reusable color values to the token
or chart-theme boundary instead of adding more exceptions.

---

## Component Entry Points

Feature and page code must import UI components from the public `src/ui/*`
entry layer:

```ts
import { Button } from "@/ui/Button";
import { Dialog } from "@/ui/Dialog";
import { Badge } from "@/ui/Badge";
import { Separator } from "@/ui/Separator";
```

Only files inside `src/ui/` may import from `@/ui/shadcn/*`. Treat
`src/ui/shadcn/*` as the internal primitive layer: it owns Radix/shadcn details,
variants, and primitive class composition. If feature code needs a primitive
shape that the public wrapper does not expose, extend the public wrapper first,
then migrate the consumer.

### Available Components

**shadcn primitives** (in `src/ui/shadcn/`): accordion, alert, badge, button,
card, dialog, dropdown-menu, input, label, popover, progress, radio-group,
scroll-area, select, separator, sheet, skeleton, spinner, switch, tab-list,
textarea, tooltip.

**Public wrappers** (in `src/ui/`): all of the above, plus higher-level composed
components: ConfirmDialog, CodeEditor, EmptyState, ErrorState, FormField,
MobileNav, PageHeader, Popover (controlled), QueryStateView, SettingsRow,
Sidebar, Tooltip (smart trigger).

---

## Font Strategy

- **Display**: Plus Jakarta Sans with DM Sans and system fallbacks for compact
  route headers and premium navigation surfaces.
- **Sans-serif**: DM Sans / Plus Jakarta Sans with system fallbacks for app body
  text. The app loads these webfonts from `index.html`; keep system fallbacks
  first-class so offline or blocked font requests still render cleanly.
- **Monospace**: JetBrains Mono with extended Linux-compatible fallbacks (DejaVu
  Sans Mono, Liberation Mono) defined in `tailwind.config.ts` `fontFamily.mono`.
- Font rendering optimized: `font-synthesis: none`, `text-rendering:
  optimizeLegibility`, `-webkit-font-smoothing: antialiased`

## Typography Scale and Title Hierarchy

Typography in product pages must be compact, explicit, and reviewable. Do not
rely on browser heading defaults or the global `h1`-`h6` base rule to create
visual hierarchy; every visible heading/title should carry an intentional
Tailwind size, weight, and semantic text color.

| UI role | Required size / weight | Usage rules |
|---------|------------------------|-------------|
| Route page title | `font-display text-[22px] font-semibold text-foreground` via `PageHeader` | Use `src/ui/PageHeader` for route-level pages. Do not hand-roll route `h1` styles unless the page cannot use `PageHeader`; if hand-rolled, match this scale. Legacy `text-2xl` page titles are migration debt, not a new pattern. |
| Route subtitle | `text-xs sm:text-sm text-muted-foreground` | One short line under `PageHeader`; avoid paragraph-style explanations in dense app surfaces. |
| Major panel/card title | `text-sm font-semibold text-foreground` | Required for module titles such as filters, settings groups, request-log panels, chart panels, and workflow sections. Module titles must be visibly bold (`font-semibold` or `font-bold`) and must never be `font-normal` or plain `font-medium`. |
| Entity/card primary name | `text-base font-semibold text-foreground` | Use for provider names, server names, skill names, prompt names, and other primary entities inside cards or lists. Use `text-sm font-semibold` in dense rows where `text-base` would crowd controls. |
| Subsection/table header | `text-xs font-semibold uppercase tracking-wide text-muted-foreground` | Use for grouped metadata labels, table headers, and small section dividers. Keep tracking non-negative. |
| Form/settings label | `text-sm font-medium text-foreground` or `text-secondary-foreground` | Labels are controls, not section titles; use `font-medium` here, but do not reuse this weight for module titles. |
| Body/content text | `text-sm text-foreground` | Default readable content size for desktop product surfaces. |
| Metadata/help text | `text-xs text-muted-foreground` | Use `font-normal` by default; add `font-medium` only for badges, compact labels, or clickable metadata. |
| Metric value | `text-lg` or `text-xl` with `font-semibold tabular-nums text-foreground` | Use `text-xl` only for top-level summary cards; use `text-lg` or `text-sm` in compact panels. |
| Dialog/sheet title | Public dialog/sheet primitives | Use shared primitives instead of redefining title sizes per dialog. Override only for a product-specific reason. |

### Typography Review Rules

- New module, panel, chart, settings-group, and card-section titles must include
  an explicit bold weight: `font-semibold` by default, `font-bold` only when the
  surrounding surface already uses stronger title emphasis.
- Do not use `text-lg`, `text-xl`, or larger for ordinary module titles inside
  dense product pages. Reserve larger sizes for route headers, entity hero rows,
  or metric values.
- Do not use `font-normal` or plain `font-medium` for module titles. If a title
  visually blends into metadata, it is a hierarchy bug.
- Do not introduce viewport-scaled font sizes, negative letter spacing, or
  arbitrary typography values in feature code. Existing public primitives own
  any exceptions needed for the current design system.
- Global CSS may set heading font family and semantic color, but must not force
  heading `font-weight`, `letter-spacing`, or size. Page and component classes
  own those values so `PageHeader` and module-title standards remain effective.
- Title text must fit the surface: use `min-w-0`, `truncate`, `break-words`, or
  responsive wrapping where long provider/model/workspace names can appear.

---

## Desktop Density

This is a Tauri desktop app targeting 1080p+ screens:
- Control height baseline: `h-8` (32px) for buttons and inputs via public
  `src/ui/*` primitives
- Compact toolbar gaps: `gap-2`; form rows usually use `gap-2` or `gap-3`
- Page section gaps: `gap-4` for dense panels, `gap-6` for major route regions
- Card padding: `p-3 sm:p-4` (sm) or `p-4 sm:p-5 md:p-6` (md)
- Sidebar: always visible, 232px wide (`w-[232px]`)
- Avoid marketing-page density in product surfaces: no oversized hero type,
  decorative panels, or large empty bands unless the route explicitly needs an
  empty state.

### Spacing, Radius, Typography, and Copy

- Use semantic surfaces (`bg-card`, `bg-muted`, `bg-secondary`) with
  `border-border`; do not recreate light/dark pairs with `slate-*`.
- New reusable surfaces should use `rounded-lg` or smaller. Public primitive
  surfaces may use the premium 10px radius directly when matching `--radius`.
  Existing `rounded-xl` / `rounded-2xl` usages are migration debt, not a pattern
  to copy.
- Body text defaults to `text-sm`; metadata/help text uses `text-xs` plus
  `text-muted-foreground`; title sizing and weights must follow the typography
  hierarchy above.
- Do not scale font size with viewport width or use negative letter spacing.
- UI copy should be concise Simplified Chinese for product actions and states;
  keep protocol names, CLI names, provider names, and code identifiers in their
  canonical spelling. Button labels should name the command, not explain the
  feature.

---

## Inline Styles

Inline styles are allowed for runtime geometry and third-party integration
values that Tailwind cannot express cleanly:

- CSS custom properties
- measured width/height
- virtualized or draggable transforms
- chart library geometry and tooltip adapters

Do not use inline `color`, `backgroundColor`, `fontSize`, `borderRadius`,
`boxShadow`, `fill`, or `stroke` in ordinary UI. Use Tailwind classes, design
tokens, or a shared chart/theme helper instead.

---

## Migration Status

The UI layer (`src/ui/` and `src/ui/shadcn/`) has been fully migrated to
semantic tokens. Page and feature components still contain hardcoded slate-\*
classes and should be migrated incrementally (3-5 files per PR, one semantic
category at a time).

### Legacy Bridge

`globals.css` and `tailwind.config.ts` still contain deprecated legacy tokens
(`--color-bg-primary`, `bg-bg-primary`, `text-text-theme-*`, `border-border-theme`)
for backward compatibility. New code must not use them. They will be removed once
all consumers are migrated.

### Migration Guardrails

- Keep migrations surgical: replace style drift in one small surface at a time,
  and avoid route-wide rewrites unless behavior is already being changed for a
  product reason.
- Do not introduce a new UI library or a parallel primitive folder.
- Do not remove the legacy bridge until all consumers are migrated and the
  removal has its own task.
- Preserve behavior, layout hierarchy, focus behavior, and user-facing copy
  unless the PRD explicitly asks for a change.
- Run the available frontend checks after style work (`pnpm lint`,
  `pnpm typecheck`, and targeted component tests). If a style drift script is
  added later, run it for token migration batches.
