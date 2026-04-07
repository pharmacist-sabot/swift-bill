#!/usr/bin/env node
/**
 * gen-icons.js — Swift Bill app-icon generator
 * 
 * Usage:
 *   node scripts/gen-icons.js          # Normal mode
 *   node scripts/gen-icons.js --silent # Quiet mode
 */

"use strict";

const { Resvg } = require("@resvg/resvg-js");
const fs = require("fs");
const path = require("path");
const { execSync } = require("child_process");

// Config & Paths
const ROOT = path.resolve(__dirname, "..");
const ICONS_DIR = path.join(ROOT, "src-tauri", "icons");
const SOURCE_PNG = path.join(ICONS_DIR, "icon.png");
const IS_SILENT = process.argv.includes("--silent");

// Helper: Logger
const log = (msg) => !IS_SILENT && console.log(msg);
const error = (msg) => console.error(msg);

// Icon SVG
// Note: Using viewBox="0 0 24 24" with 1024px output.
// Ensure the SVG content has sufficient padding or stroke-width for scaling.
const ICON_SVG = `<svg width="1024" height="1024" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
  <defs>
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
  <g filter="url(#softShadow)">
    <path d="M10 4H18C19.1 4 20 4.9 20 6V16" stroke="#FDD835" stroke-width="2.2" stroke-linecap="round"/>
    <path d="M7 7H15C16.1 7 17 7.9 17 9V19" stroke="#FBC02D" stroke-width="2.5" stroke-linecap="round"/>
    <rect x="4" y="10" width="10" height="11" rx="1.5" stroke="#FFFFFF" stroke-width="2.8" fill="#fcfcfc"/>
    <circle cx="7" cy="13.5" r="1.3" fill="#EF5350"/>
    <circle cx="7" cy="17.5" r="1.3" fill="#EF5350"/>
    <path d="M9.8 13.5H11.8M9.8 17.5H11.8" stroke="#EF5350" stroke-width="1.8" stroke-linecap="round"/>
  </g>
</svg>`;

// Main Execution
(async () => {
  try {
    log("🎨 Rendering SVG → icon.png (1024×1024)…");

    // 1. Render PNG
    const resvg = new Resvg(ICON_SVG, {
      fitTo: { mode: "width", value: 1024 },
      imageRendering: 1, // High quality
      shapeRendering: 2,
      textRendering: 2,
    });
    const pngBuffer = resvg.render().asPng();

    fs.mkdirSync(ICONS_DIR, { recursive: true });
    fs.writeFileSync(SOURCE_PNG, pngBuffer);
    log(`   ✓ Saved ${path.basename(SOURCE_PNG)} (${Math.round(pngBuffer.length / 1024)} KB)`);

    // 2. Generate Tauri Icons
    log("\n📦 Running tauri icon generator…");
    execSync(`npm run tauri -- icon "${SOURCE_PNG}"`, {
      cwd: ROOT,
      stdio: IS_SILENT ? "pipe" : "inherit",
      timeout: 120_000,
    });

    log("\n✅ All icons generated successfully in src-tauri/icons/");
    log("   Rebuild the app to apply: npm run tauri -- build");

  } catch (err) {
    error("\n❌ Process failed:");
    error(err.message || err);
    process.exit(1);
  }
})();
