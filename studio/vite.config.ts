import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

export default defineConfig({
  plugins: [react(), tailwindcss()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    // Web / remote-lab mode: forward RPC to `npm run server`
    proxy: { "/api": "http://127.0.0.1:7878" },
    watch: { ignored: ["**/crates/**", "**/target/**"] },
  },
  build: {
    target: "es2022",
    chunkSizeWarningLimit: 1500,
  },
});
