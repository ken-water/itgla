const { test, expect } = require("@playwright/test");

for (const width of [390, 768, 1440]) {
  test(`free-first help and plan boundaries at ${width}px`, async ({ page, request }) => {
    await page.setViewportSize({ width, height: 900 });
    const errors = [];
    page.on("pageerror", e => errors.push(e.message));
    for (const path of ["/getting-started.html", "/plans.html"]) {
      await page.goto(path);
      await expect(page.locator("h1")).toHaveCount(1);
      await expect(page.locator("main")).toBeVisible();
      await expect(page.locator("link[rel='canonical']")).toHaveAttribute("href", `https://itgla.com${path}`);
      expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth)).toBeLessThanOrEqual(0);
      const hrefs = await page.locator("main a[href^='/']").evaluateAll(nodes => nodes.map(n => n.getAttribute("href")).filter(href => !href.includes("#")));
      for (const href of new Set(hrefs)) expect((await request.get(href)).ok(), href).toBeTruthy();
    }
    await expect(page.getByRole("heading", { name: "Pro" })).toBeVisible();
    await expect(page.locator("main")).toContainText("neither is available for purchase");
    const ratio = await page.locator("main .button-primary").evaluate(button => {
      const css = getComputedStyle(button);
      const luminance = color => {
        const c = color.match(/[\d.]+/g).slice(0, 3).map(n => {
          const v = Number(n) / 255;
          return v <= .04045 ? v / 12.92 : ((v + .055) / 1.055) ** 2.4;
        });
        return c[0] * .2126 + c[1] * .7152 + c[2] * .0722;
      };
      const a = luminance(css.color), b = luminance(css.backgroundColor);
      return (Math.max(a, b) + .05) / (Math.min(a, b) + .05);
    });
    expect(ratio).toBeGreaterThanOrEqual(4.5);
    await page.goto("/getting-started.html#local-preview");
    await expect(page.locator("#local-preview")).toContainText("current 0.6.3 desktop release includes");
    await expect(page.locator("#local-preview")).toContainText("no closed-app notifications");
    expect(errors).toEqual([]);
  });
}

test("new public pages agree with sitemap, llms and entry links", async ({ page, request }) => {
  const sitemap = await (await request.get("/sitemap.xml")).text();
  const llms = await (await request.get("/llms.txt")).text();
  for (const path of ["/getting-started.html", "/plans.html"]) {
    expect(sitemap).toContain(`https://itgla.com${path}`);
    expect(llms).toContain(`https://itgla.com${path}`);
  }
  for (const path of ["/", "/downloads.html"]) {
    await page.goto(path);
    await expect(page.locator("main a[href='/getting-started.html']")).toBeVisible();
    await expect(page.locator("main a[href='/plans.html']")).toBeVisible();
  }
});

test("plan matrix distinguishes current and planned features", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto("/plans.html");
  await expect(page.locator(".plan-table-hint")).toBeVisible();
  const table = page.getByRole("table", { name: /Capabilities by plan/ });
  await expect(table.getByRole("columnheader").first()).toHaveText("Capability");
  await expect(table.locator(".plan-name")).toHaveText(["Free", "Pro", "Team"]);
  await expect(table.getByRole("rowheader", { name: "Full local database backup" })).toBeVisible();
  await expect(table).toContainText("Available in 0.6.3");
  await expect(table).not.toContainText("Local preview; pending release");
  await expect(table).toContainText("Planned; not available");
  await expect(table).toContainText("API and automation");
  await expect(page.getByText("Prices and launch dates have not been announced.")).toBeVisible();
  expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth)).toBeLessThanOrEqual(0);
  const region = page.getByRole("region", { name: "Plan comparison table" });
  expect(await region.evaluate(el => el.scrollWidth > el.clientWidth)).toBeTruthy();
});
