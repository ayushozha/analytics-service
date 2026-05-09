import { defineConfig } from "tsup";

export default defineConfig([
  {
    entry: [
      "src/index.ts",
      "src/server.ts",
      "src/react.ts",
      "src/vue.ts",
      "src/next.ts",
      "src/react-native.ts",
    ],
    format: ["cjs", "esm"],
    dts: true,
    clean: true,
    external: ["react", "vue"],
    splitting: false,
  },
  {
    entry: { "pulse.min": "src/auto.ts" },
    format: ["iife"],
    minify: true,
    clean: false,
    // No globalName — the IIFE is self-executing and doesn't need a global wrapper
  },
]);
