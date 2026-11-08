import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

export default defineConfig({
  plugins: [react()],
  // Relative asset paths, so the built page also works from a sub-folder or GitHub Pages.
  base: "./",
});
