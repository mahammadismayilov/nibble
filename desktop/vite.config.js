import { defineConfig } from "vite";
import path from "path";
import fs from "fs";

export default defineConfig({
  // The desktop shell deliberately builds the same configurator as the hosted
  // WebHID app, so driver behaviour cannot drift between the two surfaces.
  root: path.resolve(__dirname, "../webapp"),
  publicDir: false,
  plugins: [
    {
      name: "copy-assets",
      closeBundle() {
        const srcDir = path.resolve(__dirname, "../webapp/assets");
        const destDir = path.resolve(__dirname, "dist/assets");
        if (fs.existsSync(srcDir)) {
          fs.mkdirSync(destDir, { recursive: true });
          fs.cpSync(srcDir, destDir, { recursive: true });
        }
      },
    },
  ],
  build: {
    outDir: path.resolve(__dirname, "dist"),
    emptyOutDir: true,
  },
  server: {
    port: 1420,
    strictPort: true,
    fs: {
      allow: [".."],
    },
  },
});
