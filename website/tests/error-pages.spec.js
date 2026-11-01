const { expect, test } = require("@playwright/test");
const path = require("node:path");

for (const viewport of [{ width: 390, height: 844 }, { width: 1440, height: 900 }]) {
  test(`static error pages remain readable at ${viewport.width}px`, async ({ page }) => {
    await page.setViewportSize(viewport);
    for (const status of [400, 403, 404, 500, 502, 503, 504]) {
      await page.goto(`/${status}.html`);
      await expect(page.locator("main .eyebrow")).toHaveText(String(status));
      await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
      await expect(page.getByRole("link", { name: "Back to ITGLA", exact: true })).toHaveAttribute("href", "/");
      await expect(page.locator('meta[name="robots"]')).toHaveAttribute("content", "noindex, nofollow");
      expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth)).toBeLessThanOrEqual(0);
      expect(await page.locator("script").count()).toBe(0);
    }
    await page.screenshot({ path: path.resolve(__dirname, `../../output/http-errors-${viewport.width}.png`) });
  });
}

test("error instructions remain readable without external styles or JavaScript", async ({ browser }) => {
  const context = await browser.newContext({ javaScriptEnabled: false, viewport: { width: 390, height: 844 } });
  try {
    const page = await context.newPage();
    await page.route("**/styles.css*", (route) => route.abort());
    for (const status of [400, 403, 404, 500, 502, 503, 504]) {
      await page.goto(`/${status}.html`);
      await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
      await expect(page.getByRole("link", { name: "Back to ITGLA", exact: true })).toBeVisible();
      expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth)).toBeLessThanOrEqual(0);
    }
  } finally {
    await context.close();
  }
});
