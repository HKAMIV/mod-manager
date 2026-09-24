import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import path from "path";
import { fileURLToPath } from "url";
import { readFileSync } from "fs";

// This is an ESM config ("type": "module"), so __dirname is not defined.
// Derive the project root from the config file's own URL instead.
const projectRoot = path.dirname(fileURLToPath(import.meta.url));

const host = process.env.TAURI_DEV_HOST;

// Read the app version from package.json so the UI always shows the real
// current version without a hardcoded string that drifts out of date.
const pkgVersion = JSON.parse(
  readFileSync(path.resolve(projectRoot, "package.json"), "utf-8")
).version as string;

export default defineConfig({
  root: projectRoot,
  plugins: [react()],
  clearScreen: false,
  define: {
    __APP_VERSION__: JSON.stringify(pkgVersion),
  },
  resolve: {
    alias: {
      "@": path.resolve(projectRoot, "./src"),
    },
  },
  server: {
    port: 5173,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
  envPrefix: ["VITE_", "TAURI_ENV_*"],
  build: {
    // Use relative paths so the Tauri custom protocol (tauri://localhost)
    // resolves assets correctly on all WebKit versions — older WebKitGTK
    // on SteamOS/Arch doesn't handle absolute `/assets/...` paths properly
    // under the custom protocol, causing a white screen.
    target:
      process.env.TAURI_ENV_PLATFORM == "windows" ? "chrome105" : "safari13",
    minify: !process.env.TAURI_ENV_DEBUG ? "esbuild" : false,
    sourcemap: !!process.env.TAURI_ENV_DEBUG,
  },
  base: "",
});
