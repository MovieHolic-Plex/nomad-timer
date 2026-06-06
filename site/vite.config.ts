import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  build: {
    outDir: "dist",
    sourcemap: true
  },
  preview: {
    port: 4173,
    strictPort: true
  },
  server: {
    port: 5173,
    strictPort: true
  },
  test: {
    environment: "jsdom"
  }
});
