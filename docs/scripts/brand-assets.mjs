// Run from any directory: node docs/scripts/brand-assets.mjs
// Export approved artwork and real recording frames; no image generation here.
import sharp from "sharp";
import { fileURLToPath } from "node:url";
import { mkdir, readFile, writeFile, copyFile } from "node:fs/promises";
const file = (path) => fileURLToPath(new URL(path, import.meta.url));
await mkdir(file("../public/brand/"), { recursive: true });
await sharp(file("../../assets/brand/mascot-assembler.png"))
  .resize({ width: 960, kernel: "nearest" })
  .webp({ lossless: true })
  .toFile(file("../public/brand/mascot-assembler.webp"));
const hero = file("../../assets/brand/readme-cover.png");
await sharp(hero)
  .resize(1536)
  .webp({ quality: 85 })
  .toFile(file("../public/brand/hero.webp"));
await sharp(hero)
  .resize(1200, 630, { fit: "cover" })
  .png({ palette: true, colours: 256, compressionLevel: 9 })
  .toFile(file("../public/social-preview.png"));
for (const [name, time] of [
  ["todo-app", 6000],
  ["mouse", 6000],
  ["table", 8400],
  ["router", 6000],
]) {
  const source = file(`../public/recordings/${name}.gif`);
  const metadata = await sharp(source).metadata();
  let elapsed = 0;
  let page = 0;
  for (const delay of metadata.delay ?? []) {
    if (elapsed >= time) break;
    elapsed += delay;
    page++;
  }
  page = Math.min(page, (metadata.pages ?? 1) - 1);
  await sharp(source, { page })
    .webp({ quality: 90 })
    .toFile(file(`../public/brand/${name}-poster.webp`));
}
await copyFile(
  file("../src/assets/logo.svg"),
  file("../public/brand/logo.svg"),
);
const svg = await readFile(file("../src/assets/logo.svg"), "utf8");
const mark = svg
  .slice(svg.indexOf("<svg"))
  .replace("<svg ", '<svg x="32" y="32" width="112" height="116" ');
for (const [theme, background, foreground] of [
  ["light", "#FCF8EF", "#064A4F"],
  ["dark", "#081518", "#FCF8EF"],
]) {
  const lockup = `<svg xmlns="http://www.w3.org/2000/svg" width="760" height="180" viewBox="0 0 760 180"><rect width="760" height="180" fill="${background}"/>${mark}<text x="178" y="97" fill="${foreground}" font-family="Arial, sans-serif" font-size="52" font-weight="700" letter-spacing="-2">Ratatui Kit</text><text x="180" y="130" fill="${foreground}" font-family="Arial, sans-serif" font-size="13" letter-spacing="3">TERMINAL UI. COMPOSED.</text></svg>`;
  await writeFile(file(`../public/brand/wordmark-${theme}.svg`), lockup);
}
console.log("Exported brand artwork, wordmarks, and four recording posters.");
