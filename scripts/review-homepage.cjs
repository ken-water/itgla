const { chromium } = require(process.env.PLAYWRIGHT_MODULE || 'playwright');
const fs = require('fs');
const path = require('path');
(async () => {
  const out = path.join(process.env.TEMP, 'itgla-web-20261026');
  fs.mkdirSync(out, { recursive: true });
  const browser = await chromium.launch({ channel: 'chrome' });
  const results = [];
  for (const width of [1440, 768, 390]) {
    const page = await browser.newPage({ viewport: { width, height: 900 } });
    const errors = [];
    page.on('pageerror', e => errors.push(e.message));
    await page.goto('http://127.0.0.1:4183/', { waitUntil: 'networkidle' });
    await page.screenshot({ path: path.join(out, `home-${width}.png`), fullPage: true });
    const state = await page.evaluate(() => ({
      overflow: document.documentElement.scrollWidth > innerWidth,
      imagesLoaded: [...document.images].every(i => i.complete && i.naturalWidth > 0),
      heading: document.querySelector('h1').textContent,
      smallTargets: [...document.querySelectorAll('.button')].filter(e => e.getBoundingClientRect().height < 44).length
    }));
    results.push({ width, ...state, errors });
    await page.close();
  }
  await browser.close();
  fs.writeFileSync(path.join(out, 'results.json'), JSON.stringify(results, null, 2));
  console.log(JSON.stringify(results, null, 2));
  if (results.some(r => r.overflow || !r.imagesLoaded || r.errors.length || r.smallTargets)) process.exitCode = 1;
})().catch(e => { console.error(e); process.exit(1); });
