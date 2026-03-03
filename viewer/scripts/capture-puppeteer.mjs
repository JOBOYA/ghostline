/**
 * Ghostline Viewer — Puppeteer Screenshot Capture
 * Usage: node viewer/scripts/capture-puppeteer.mjs
 *
 * Captures 3 variants:
 *   1. empty-state.png   — drop zone, 1200×675
 *   2. hero-timeline.png — loaded run, 1200×675
 *   3. detail-panel.png  — run + detail panel, 1200×800
 */

import puppeteer from 'puppeteer';
import http from 'http';
import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const DIST_DIR   = path.resolve(__dirname, '../dist');
const OUT_DIR    = path.resolve(__dirname, '../../docs/screenshots');
const DEMO_FILE  = path.resolve(__dirname, '../../examples/demo.ghostline');
const PORT       = 7788;

fs.mkdirSync(OUT_DIR, { recursive: true });

// ---------- simple static server ----------
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

const MIME = {
  '.html': 'text/html',
  '.js': 'application/javascript',
  '.css': 'text/css',
  '.svg': 'image/svg+xml',
  '.png': 'image/png',
  '.ico': 'image/x-icon',
  '.woff2': 'font/woff2',
};

function serveStatic(req, res, opts = {}) {
  let filePath = path.join(DIST_DIR, req.url === '/' ? '/index.html' : req.url);

  // Strip query strings
  filePath = filePath.split('?')[0];

  if (!fs.existsSync(filePath)) {
    // SPA fallback — serve modified index
    filePath = path.join(DIST_DIR, 'index.html');
  }

  const ext = path.extname(filePath);
  const contentType = MIME[ext] || 'application/octet-stream';

  if (filePath.endsWith('index.html') && opts.injectData) {
    let html = fs.readFileSync(filePath, 'utf8');
    html = html.replace(
      '</body>',
      `<script id="ghostline-data" type="text/plain" data-filename="demo.ghostline">${opts.demoBase64}</script></body>`
    );
    res.writeHead(200, { 'Content-Type': 'text/html' });
    res.end(html);
    return;
  }

  res.writeHead(200, { 'Content-Type': contentType });
  fs.createReadStream(filePath).pipe(res);
}

// ---------- main ----------
async function main() {
  const demoBase64 = fs.existsSync(DEMO_FILE)
    ? fs.readFileSync(DEMO_FILE).toString('base64')
    : null;

  // Server 1: plain (for empty state)
  const server1 = http.createServer((req, res) => serveStatic(req, res));
  await new Promise((r) => server1.listen(PORT, r));

  // Server 2: with injected demo data (for loaded state)
  const server2 = http.createServer((req, res) =>
    serveStatic(req, res, { injectData: !!demoBase64, demoBase64 })
  );
  await new Promise((r) => server2.listen(PORT + 1, r));

  const browser = await puppeteer.launch({
    executablePath: '/usr/bin/chromium',
    args: ['--no-sandbox', '--disable-setuid-sandbox', '--disable-dev-shm-usage'],
    headless: true,
  });

  // ── 1. Empty state ──────────────────────────────────────────────
  console.log('📸 Capturing empty-state.png …');
  {
    const page = await browser.newPage();
    await page.setViewport({ width: 1200, height: 675 });
    await page.goto(`http://localhost:${PORT}/`, { waitUntil: 'networkidle0' });
    await sleep(800);
    await page.screenshot({ path: path.join(OUT_DIR, 'empty-state.png') });
    await page.close();
    console.log('  ✅ empty-state.png');
  }

  if (demoBase64) {
    // ── 2. Hero timeline (demo data loaded) ──────────────────────────
    console.log('📸 Capturing hero-timeline.png …');
    {
      const page = await browser.newPage();
      await page.setViewport({ width: 1200, height: 675 });
      await page.goto(`http://localhost:${PORT + 1}/`, { waitUntil: 'networkidle0' });
      // Wait for ReactFlow to render + fitView
      await sleep(2500);
      await page.screenshot({ path: path.join(OUT_DIR, 'hero-timeline.png') });
      await page.close();
      console.log('  ✅ hero-timeline.png');
    }

    // ── 3. Detail panel ──────────────────────────────────────────────
    console.log('📸 Capturing detail-panel.png …');
    {
      const page = await browser.newPage();
      await page.setViewport({ width: 1200, height: 800 });
      await page.goto(`http://localhost:${PORT + 1}/`, { waitUntil: 'networkidle0' });
      await sleep(2500);
      // Select first frame (j key) and open detail (Enter)
      await page.keyboard.press('j');
      await sleep(300);
      await page.keyboard.press('Enter');
      await sleep(600);
      await page.screenshot({ path: path.join(OUT_DIR, 'detail-panel.png') });
      await page.close();
      console.log('  ✅ detail-panel.png');
    }
  } else {
    console.warn('⚠️  No demo.ghostline found — skipping loaded-state screenshots');
  }

  await browser.close();
  server1.close();
  server2.close();

  console.log(`\n✅ Done. Screenshots saved to: ${OUT_DIR}`);
}

main().catch((e) => { console.error(e); process.exit(1); });
