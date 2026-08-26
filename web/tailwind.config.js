/** @type {import("tailwindcss").Config} */
const colors = require("tailwindcss/colors");

module.exports = {
  content: [
    "./pages/**/*.{js,ts,jsx,tsx}",
    "./components/**/*.{js,ts,jsx,tsx}",
    "./context/**/*.{js,ts,jsx,tsx}",
    "./wasm-upload/**/*.{js,ts,jsx,tsx}",
  ],
  // Status/severity indicators pick their color classes from a fixed palette at
  // runtime. The safelist ensures Tailwind never purges these even when class
  // names are assembled dynamically.
  safelist: [
    {
      pattern:
        /^(bg|text|border|ring)-(rose|amber|emerald|cyan|sky|violet|pink|indigo|orange|blue|green|slate)-(50|100|200|300|400|500|600|700|800|900|950)$/,
    },
  ],
  theme: {
    extend: {
      colors: {
        // ──────────────────────────────────────────────────────────────
        // Design tokens (WEB-19, #105)
        //
        // Semantic tokens below are sourced from CSS custom properties
        // declared in `web/styles/variables.css` under `:root`. Using
        // `var(--token, <fallback>)` keeps every utility class (`bg-primary`,
        // `text-foreground`, `border-border`, …) reactive to a single
        // rebrand: just change the value in `variables.css`.
        //
        // Numeric RGB channels (`<token>-rgb`) are also exposed so future
        // alpha utilities (`bg-primary/40`) can be added without touching
        // this file.
        // ──────────────────────────────────────────────────────────────
        primary: {
          DEFAULT: "var(--color-primary, #2563eb)",
          hover: "var(--color-primary-hover, #1d4ed8)",
          rgb: "37 99 235",
        },
        secondary: {
          DEFAULT: "var(--color-secondary, #64748b)",
          hover: "var(--color-secondary-hover, #475569)",
        },
        success: "var(--color-success, #16a34a)",
        warning: "var(--color-warning, #f59e0b)",
        danger:  "var(--color-danger,  #dc2626)",
        canvas:  "var(--color-background, #ffffff)",
        surface: "var(--color-surface,    #f8fafc)",
        line: {
          DEFAULT: "var(--color-border, #e2e8f0)",
          subtle:  "var(--color-border-subtle, #e2e8f0)",
        },
        body: {
          DEFAULT: "var(--color-text,       #0f172a)",
          muted:   "var(--color-text-muted, #64748b)",
        },

        // ── Existing WCAG-AA contrast overrides — preserved from WEB-59 (#192).
        // All values below target dark backgrounds (slate-950 / slate-900).
        // Minimum required contrast ratio: 4.5:1 normal text, 3:1 large text.
        slate: {
          ...colors.slate,
          // slate-300 on slate-950 → ~10.7:1 (AAA)  ✓
          // slate-400 on slate-950 →  ~6.8:1 (AA)   ✓
          400: colors.slate[300], // #cbd5e1
          500: colors.slate[400], // #94a3b8
        },
        gray: {
          ...colors.gray,
          // gray-300 on slate-950 → ~10.3:1 (AAA)  ✓
          // gray-400 on slate-950 →  ~6.6:1 (AA)   ✓
          // gray-500 default #6b7280 → 4.6:1 on white ✓ (kept)
          // gray-600 default #4b5563 → 3.9:1 on white ✗ → map to gray-500
          400: colors.gray[300],  // #d1d5db
          500: colors.gray[400],  // #9ca3af
          600: colors.gray[500],  // #6b7280 — 4.6:1 on white ✓
        },
        zinc: {
          ...colors.zinc,
          // zinc-400 on slate-950 → ~6.9:1 (AA)  ✓
          400: colors.zinc[300],  // #d4d4d8
          500: colors.zinc[400],  // #a1a1aa
        },
      },
      spacing: {
        120: "30rem",
        // Brand-aligned spacing tokens sourced from CSS vars.
        "section":   "var(--space-section,   4rem)",
        "component": "var(--space-component, 1.5rem)",
      },
      borderRadius: {
        "4xl": "2rem",
        "s-2xl": "1rem 0 0 1rem",
        "e-2xl": "0 1rem 1rem 0",
        // Brand-aligned radius tokens sourced from CSS vars.
        // Namespaced under `card-*` / `pill` so they do NOT shadow Tailwind's
        // built-in `sm` / `md` / `lg` / `xl` / `full` keys (a regression
        // caught by code-review for #105). Use as `rounded-card-sm`,
        // `rounded-pill`, etc.
        "card-sm": "var(--radius-sm, 6px)",
        "card-md": "var(--radius-md, 10px)",
        "card-lg": "var(--radius-lg, 14px)",
        "card-xl": "var(--radius-xl, 18px)",
        "pill":    "var(--radius-full, 9999px)",
      },
      boxShadow: {
        // Namespaced under `card-*` to avoid shadowing Tailwind's built-in
        // `sm` / `md` / `lg`. Use as `shadow-card-sm`, etc.
        "card-sm": "var(--shadow-sm, 0 1px 2px rgba(0,0,0,.05))",
        "card-md": "var(--shadow-md, 0 4px 12px rgba(0,0,0,.08))",
        "card-lg": "var(--shadow-lg, 0 10px 24px rgba(0,0,0,.12))",
      },
    },
  },
  plugins: [],
};
