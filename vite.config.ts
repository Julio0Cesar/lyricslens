import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(async () => ({
  plugins: [react(), tailwindcss()],

  // O ambiente padrão é `node`: as funções puras não precisam de DOM e rodam
  // mais rápido sem ele. Teste de componente pede `jsdom` no topo do arquivo,
  // com `// @vitest-environment jsdom` — assim o custo fica em quem usa.
  test: {
    setupFiles: ["./vitest.setup.ts"],
    // A API do Tauri não existe fora do webview; o setup a substitui, e sem
    // isolar cada arquivo o registro de comandos vazaria de um teste para o
    // outro.
    restoreMocks: true,
  },

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
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
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },
}));
