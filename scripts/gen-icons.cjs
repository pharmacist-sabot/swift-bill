#!/usr/bin/env node
/**
 * gen-icons.js — Swift Bill app-icon generator
 *
 * Renders the Swift Bill SVG logo at 1024×1024 (with a deep-blue gradient
 * background so that the white/yellow artwork is always visible), then hands
 * the resulting PNG to the Tauri CLI which produces every required format:
 *
 *   icons/32x32.png
 *   icons/128x128.png
 *   icons/128x128@2x.png
 *   icons/icon.icns   (macOS)
 *   icons/icon.ico    (Windows)
 *   icons/Square*.png (Windows Store)
 *   icons/StoreLogo.png
 *
 * Usage:
 *   node scripts/gen-icons.js
 *
 * Prerequisites (already in devDependencies):
 *   @resvg/resvg-js
 */

"use strict";

const { Resvg } = require("@resvg/resvg-js");
const fs = require("fs");
const path = require("path");
const { execSync } = require("child_process");

// ── Paths ─────────────────────────────────────────────────────────────────────
const ROOT = path.resolve(__dirname, "..");
const ICONS_DIR = path.join(ROOT, "src-tauri", "icons");
const SOURCE_PNG = path.join(ICONS_DIR, "icon.png");

// ── Icon SVG ──────────────────────────────────────────────────────────────────
// The artwork is 100% identical to the SVG provided by the designer.
// We only add:
//   1. A 1024×1024 canvas (width/height attributes)
//   2. A deep-blue gradient background rect so the white elements are visible
//   3. Adjusted filter stdDeviation to suit icon-scale rendering
//
// viewBox stays "-3 -3 30 30" to give the 24×24 artwork ~10 % padding on
// all sides — standard for app-icon padding guidelines.
// NO background is added — the SVG is used exactly as provided (transparent).
const ICON_SVG = `<svg
  width="1024"
  height="1024"
  viewBox="0 0 24 24"
  fill="none"
  xmlns="http://www.w3.org/2000/svg"
>
  <defs>
    <!--
      Soft drop-shadow filter.
      stdDeviation is kept small (0.5) because in the 24-unit viewBox each
      unit maps to ~42 px at 1024 px; a value of 0.5 ≈ 21 px at source
      resolution, which looks natural and scales down gracefully.
    -->
    <filter id="softShadow" x="-20%" y="-20%" width="140%" height="140%">
      <feGaussianBlur in="SourceAlpha" stdDeviation="0.5" result="blur"/>
      <feOffset in="blur" dx="0.35" dy="0.45" result="offsetBlur"/>
      <feFlood flood-color="rgba(0,0,0,0.18)" result="color"/>
      <feComposite in="color" in2="offsetBlur" operator="in" result="shadow"/>
      <feMerge>
        <feMergeNode in="shadow"/>
        <feMergeNode in="SourceGraphic"/>
      </feMerge>
    </filter>
  </defs>

  <!-- ── Original artwork — 100% identical to the provided SVG ── -->
  <g filter="url(#softShadow)">
    <path
      d="M10 4H18C19.1 4 20 4.9 20 6V16"
      stroke="#FDD835"
      stroke-width="2.2"
      stroke-linecap="round"
    />
    <path
      d="M7 7H15C16.1 7 17 7.9 17 9V19"
      stroke="#FBC02D"
      stroke-width="2.5"
      stroke-linecap="round"
    />
    <rect
      x="4" y="10" width="10" height="11"
      rx="1.5"
      stroke="#FFFFFF"
      stroke-width="2.8"
      fill="#fcfcfc"
    />
    <circle cx="7" cy="13.5" r="1.3" fill="#EF5350"/>
    <circle cx="7" cy="17.5" r="1.3" fill="#EF5350"/>
    <path
      d="M9.8 13.5H11.8M9.8 17.5H11.8"
      stroke="#EF5350"
      stroke-width="1.8"
      stroke-linecap="round"
    />
  </g>
</svg>`;

// ── Render 1024 × 1024 PNG ────────────────────────────────────────────────────
console.log("🎨  Rendering SVG → icon.png (1024 × 1024) …");

let pngBuffer;
try {
  const resvg = new Resvg(ICON_SVG, {
    fitTo: { mode: "width", value: 1024 },
    // Use high-quality image rendering
    imageRendering: 1,
    shapeRendering: 2,
    textRendering: 2,
  });
  const rendered = resvg.render();
  pngBuffer = rendered.asPng();
} catch (err) {
  console.error("❌  SVG render failed:", err.message);
  process.exit(1);
}

fs.mkdirSync(ICONS_DIR, { recursive: true });
fs.writeFileSync(SOURCE_PNG, pngBuffer);
console.log(`   ✓  Saved ${SOURCE_PNG} (${Math.round(pngBuffer.length / 1024)} KB)`);

// ── Delegate to Tauri CLI ─────────────────────────────────────────────────────
// "npm run tauri -- icon <png>" generates ALL required icon formats in one shot:
//   PNG at every size, .icns (macOS), .ico (Windows), Windows-Store squares …
console.log("\n📦  Running tauri icon generator …\n");

try {
  execSync(`npm run tauri -- icon "${SOURCE_PNG}"`, {
    cwd: ROOT,
    stdio: "inherit",
    // Give it a generous timeout (icon conversion can be slow on first run)
    timeout: 120_000,
  });
} catch (err) {
  console.error("\n❌  tauri icon command failed.");
  console.error("    Make sure @tauri-apps/cli is installed (npm install).");
  console.error("    Error:", err.message);
  process.exit(1);
}

console.log("\n✅  All icon sizes generated successfully in src-tauri/icons/");
console.log("    Rebuild the app to apply: npm run tauri -- build");
