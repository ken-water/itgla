const { expect, test } = require("@playwright/test");

const viewports = [
  { name: "mobile", width: 390, height: 844 },
  { name: "tablet", width: 768, height: 1024 },
  { name: "desktop", width: 1440, height: 900 },
];

for (const viewport of viewports) {
  test(`homepage is usable at ${viewport.name}`, async ({ page }) => {
    const consoleErrors = [];
    page.on("console", (message) => {
      if (message.type() === "error") consoleErrors.push(message.text());
    });

    await page.setViewportSize(viewport);
    await page.goto("/");

    await expect(page.getByRole("heading", { name: "ITGLA", exact: true })).toBeVisible();
    await expect(page.getByRole("link", { name: "Download Windows portable" })).toBeVisible();
    await expect(page.getByText("Current stable download: v0.1.1", { exact: false })).toBeVisible();
    await expect(page.getByRole("heading", { name: "Import your table" })).toBeVisible();
    await expect(page.getByRole("heading", { name: "Sort and filter" })).toBeVisible();
    await expect(page.getByRole("heading", { name: "Copy exactly" })).toBeVisible();
    await expect(page.locator(".relationship-demo .relation-node")).toHaveCount(3);
    await expect(page.locator(".relationship-demo .relation-line")).toHaveCount(2);

    const overflow = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
    expect(overflow).toBeLessThanOrEqual(0);
    expect(consoleErrors).toEqual([]);
  });
}

test("download page exposes current verified artifacts", async ({ page, request }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto("/downloads.html");

  await expect(page.getByRole("heading", { name: "Download ITGLA v0.1.1" })).toBeVisible();
  await expect(page.getByRole("link", { name: "Download ZIP" })).toHaveAttribute(
    "href",
    "/downloads/v0.1.1/itgla-v0.1.1-windows-x86_64.zip",
  );

  for (const path of [
    "/downloads/v0.1.1/itgla-v0.1.1-windows-x86_64.zip",
    "/downloads/v0.1.1/itgla-v0.1.1-linux-x86_64.tar.gz",
    "/downloads/v0.1.1/SHA256SUMS",
  ]) {
    const response = await request.get(path);
    expect(response.ok()).toBeTruthy();
  }

  const overflow = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
  expect(overflow).toBeLessThanOrEqual(0);
});

test("secondary documents render", async ({ page }) => {
  for (const path of ["/privacy.html", "/legal.html", "/feedback.html"]) {
    const response = await page.goto(path);
    expect(response?.ok()).toBeTruthy();
    await expect(page.locator("h1")).toBeVisible();
  }
});
