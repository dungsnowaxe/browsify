import { defineConfig } from "vite";
import stylex from "@stylexjs/unplugin";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import { browsifyDevProxyPlugin } from "./dev-proxy.mjs";

// https://vite.dev/config/
export default defineConfig({
  plugins: [
    browsifyDevProxyPlugin(),
    stylex.vite({
      useCSSLayers: true,
      unstable_moduleResolution: { type: "commonJS" },
    }),
    svelte(),
  ],
  base: "/",
  build: {
    assetsDir: "assets",
    rollupOptions: {
      output: {
        assetFileNames: "[name][extname]",
        chunkFileNames: "[name].js",
        entryFileNames: "[name].js",
      },
    },
  },
});
