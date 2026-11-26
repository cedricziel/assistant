// Bundles src/app.js with the MCP Apps SDK into one self-contained HTML file,
// dist/app.html: hosts render the app from a single resource, with no network.

import { mkdir, readFile, writeFile } from "node:fs/promises";

import { build } from "esbuild";

const { outputFiles } = await build({
  entryPoints: [new URL("./src/app.js", import.meta.url).pathname],
  bundle: true,
  format: "esm",
  target: "es2022",
  minify: true,
  write: false,
});
const script = outputFiles[0].text.replaceAll("</script", "<\\/script");
const template = await readFile(new URL("./src/app.html", import.meta.url), "utf8");
const html = template.replace(
  "<script>\n      /* APP_SCRIPT */\n    </script>",
  () => `<script type="module">${script}</script>`,
);
if (html === template) throw new Error("src/app.html has no APP_SCRIPT slot");

await mkdir(new URL("./dist/", import.meta.url), { recursive: true });
await writeFile(new URL("./dist/app.html", import.meta.url), html);
console.log(`dist/app.html: ${html.length} bytes`);
