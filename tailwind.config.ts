import type { Config } from "tailwindcss";
import typography from "@tailwindcss/typography";

export default {
  darkMode: "class",
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  theme: {
    // Extend default screens with xs breakpoint for extra-small devices
    screens: {
      xs: "475px",
      sm: "640px",
      md: "768px",
      lg: "1024px",
      xl: "1280px",
      "2xl": "1536px",
    },
    extend: {
      colors: {
        accent: {
          DEFAULT: "#0052FF",
          secondary: "#4D7CFF",
        },
        success: { DEFAULT: "#16A34A" },
        warning: { DEFAULT: "#F97316" },
        danger: { DEFAULT: "#DC2626" },
        info: { DEFAULT: "#0EA5E9" },
        // CSS variable bridge — new code can use these semantic tokens
        // while existing slate-* references are migrated incrementally.
        bg: {
          primary: "var(--color-bg-primary)",
          secondary: "var(--color-bg-secondary)",
          card: "var(--color-bg-card)",
        },
        "text-theme": {
          primary: "var(--color-text-primary)",
          secondary: "var(--color-text-secondary)",
          muted: "var(--color-text-muted)",
        },
        "border-theme": {
          DEFAULT: "var(--color-border)",
          light: "var(--color-border-light)",
        },
      },
      boxShadow: {
        card: "0 1px 3px rgba(15, 23, 42, 0.04), 0 4px 12px rgba(15, 23, 42, 0.06)",
      },
    },
  },
  plugins: [typography],
} satisfies Config;
