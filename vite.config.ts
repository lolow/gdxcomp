/// <reference types="vitest/config" />
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// Tauri expects a fixed port and the dev server not to clear the screen so its
// own logs remain visible.
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
  },
  // Produce a relative-path build so Tauri can load assets from disk.
  base: "./",
  build: {
    // Plotly + react-plotly.js are shipped as their own chunk so the main
    // entry stays small (faster first paint). The chunk is loaded on demand
    // when ChartView is mounted (React.lazy).
    rollupOptions: {
      output: {
        manualChunks: {
          plotly: ["plotly.js-basic-dist-min", "react-plotly.js"],
        },
      },
    },
  },
  test: {
    environment: "jsdom",
    globals: true,
    setupFiles: ["./src/test/setup.ts"],
  },
});
