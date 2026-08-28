/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{js,ts,jsx,tsx}"],
  darkMode: "class",
  theme: {
    extend: {
      fontFamily: {
        display: ["var(--font-display)", "Rajdhani", "sans-serif"],
        sans: ["Inter", "-apple-system", "sans-serif"],
      },
      boxShadow: {
        "glow-accent": "0 0 12px -2px var(--accent), 0 0 1px 0 var(--accent)",
        "glow-accent-lg": "0 0 32px -4px var(--accent), 0 0 2px 0 var(--accent)",
        "glow-game": "0 0 12px -2px var(--game-accent), 0 0 1px 0 var(--game-accent)",
        "glow-game-lg": "0 0 32px -4px var(--game-accent), 0 0 2px 0 var(--game-accent)",
        "glow-game2": "0 0 12px -2px var(--game-accent2), 0 0 1px 0 var(--game-accent2)",
        "glow-gold": "0 0 12px -2px var(--gold), 0 0 1px 0 var(--gold)",
        "glow-accent2": "0 0 12px -2px var(--accent-secondary)",
      },
      backgroundImage: {
        "grid-pattern":
          "linear-gradient(var(--surface-3) 1px, transparent 1px), linear-gradient(90deg, var(--surface-3) 1px, transparent 1px)",
      },
      backgroundSize: {
        grid: "32px 32px",
      },
      animation: {
        "pulse-glow": "pulse-glow 2.4s ease-in-out infinite",
        "scan-line": "scan-line 3s linear infinite",
      },
      keyframes: {
        "pulse-glow": {
          "0%, 100%": { opacity: "1" },
          "50%": { opacity: "0.55" },
        },
        "scan-line": {
          "0%": { transform: "translateY(-100%)" },
          "100%": { transform: "translateY(100%)" },
        },
      },
      colors: {
        surface: {
          0: "var(--surface-0)",
          1: "var(--surface-1)",
          2: "var(--surface-2)",
          3: "var(--surface-3)",
        },
        accent: {
          DEFAULT: "var(--accent)",
          hover: "var(--accent-hover)",
        },
        accent2: {
          DEFAULT: "var(--accent-secondary)",
          hover: "var(--accent-secondary-hover)",
        },
        gold: {
          DEFAULT: "var(--gold)",
          hover: "var(--gold-hover)",
        },
        game: {
          DEFAULT: "var(--game-accent)",
          hover: "var(--game-accent-hover)",
        },
        game2: {
          DEFAULT: "var(--game-accent2)",
          hover: "var(--game-accent2-hover)",
        },
        text: {
          primary: "var(--text-primary)",
          secondary: "var(--text-secondary)",
          muted: "var(--text-muted)",
        },
      },
    },
  },
  plugins: [],
};
