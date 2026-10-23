import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import { fileURLToPath, URL } from "node:url";

export default defineConfig({
  plugins: [vue()],
  resolve: {
    alias: {
      "@": fileURLToPath(new URL("./src", import.meta.url)),
      "@locales": fileURLToPath(new URL("../locales", import.meta.url)),
    },
  },
  server: { port: 1420, strictPort: true, fs: { allow: [".."] } },
  clearScreen: false,
});
