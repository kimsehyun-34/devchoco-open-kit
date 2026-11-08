// tsc only emits JavaScript and declarations; the stylesheet is shipped as-is.
import { copyFileSync, mkdirSync } from "node:fs";

mkdirSync(new URL("../dist", import.meta.url), { recursive: true });
copyFileSync(
  new URL("../src/styles.css", import.meta.url),
  new URL("../dist/styles.css", import.meta.url),
);
