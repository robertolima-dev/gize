import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// The API base is proxied so the browser talks same-origin in dev. Set VITE_API_URL to point
// at your running Gize backend (default http://localhost:8080).
export default defineConfig({
  plugins: [react()],
  server: {
    proxy: {
      "/api": {
        target: process.env.VITE_API_URL || "http://localhost:8080",
        changeOrigin: true,
        rewrite: (p) => p.replace(/^\/api/, ""),
      },
    },
  },
});
