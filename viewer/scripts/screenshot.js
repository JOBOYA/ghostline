/**
 * Ghostline Viewer Screenshot Script
 * Usage: npx playwright test screenshot.js
 * Requires: npx playwright install chromium
 * 
 * Captures 3 variants per screenshot spec:
 * 1. Hero (1200x675) - OG image for README/Show HN
 * 2. Timeline detail (1200x800) - with detail panel open
 * 3. Empty state (1200x675) - drop zone
 */

import { chromium } from 'playwright';
import path from 'path';
import { fileURLToPath } from 'url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const VIEWER_URL = process.env.VIEWER_URL || 'http://localhost:5173';
const OUT_DIR = path.resolve(__dirname, '../../docs/screenshots');

async function main() {
  const browser = await chromium.launch();

  // 1. Empty state
  const ctx1 = await browser.newContext({ viewport: { width: 1200, height: 675 } });
  const page1 = await ctx1.newPage();
  await page1.goto(VIEWER_URL);
  await page1.waitForTimeout(1000);
  await page1.screenshot({ path: path.join(OUT_DIR, 'empty-state.png') });
  await ctx1.close();

  // 2. Hero with demo file loaded
  const ctx2 = await browser.newContext({ viewport: { width: 1200, height: 675 } });
  const page2 = await ctx2.newPage();
  await page2.goto(VIEWER_URL);
  await page2.waitForTimeout(500);
  // Load demo file via file input
  const input = await page2.locator('input[type="file"]');
  await input.setInputFiles(path.resolve(__dirname, '../src/demo.ghostline'));
  await page2.waitForTimeout(1500); // let ReactFlow render + fitView
  await page2.screenshot({ path: path.join(OUT_DIR, 'hero-timeline.png') });

  // 3. With detail panel open — click first node then press Enter
  await page2.keyboard.press('j'); // select first frame
  await page2.waitForTimeout(200);
  await page2.keyboard.press('Enter'); // open detail
  await page2.waitForTimeout(500);
  const ctx3 = await browser.newContext({ viewport: { width: 1200, height: 800 } });
  const page3 = await ctx3.newPage();
  await page3.goto(VIEWER_URL);
  await page3.waitForTimeout(500);
  const input3 = await page3.locator('input[type="file"]');
  await input3.setInputFiles(path.resolve(__dirname, '../src/demo.ghostline'));
  await page3.waitForTimeout(1500);
  await page3.keyboard.press('j');
  await page3.waitForTimeout(200);
  await page3.keyboard.press('Enter');
  await page3.waitForTimeout(500);
  await page3.screenshot({ path: path.join(OUT_DIR, 'detail-panel.png') });

  await browser.close();
  console.log(`Screenshots saved to ${OUT_DIR}`);
}

main().catch(console.error);
