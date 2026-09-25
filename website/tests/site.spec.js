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
    await expect(page.getByRole("link", { name: "Download Windows installer" })).toBeVisible();
    await expect(page.getByText("Current stable download: v0.2.2", { exact: false })).toBeVisible();
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

  await expect(page.getByRole("heading", { name: "Download ITGLA v0.2.2" })).toBeVisible();
  await expect(page.getByRole("link", { name: "Download installer" })).toHaveAttribute(
    "href",
    "https://github.com/ken-water/itgla/releases/download/v0.2.2/itgla-v0.2.2-windows-x86_64-setup.exe",
  );

  for (const name of ["Portable ZIP", "DEB", "RPM", "AppImage", "Download DMG", "Download SHA256SUMS"]) {
    await expect(page.getByRole("link", { name, exact: true })).toBeVisible();
  }

  const overflow = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
  expect(overflow).toBeLessThanOrEqual(0);
});

test("secondary documents render", async ({ page }) => {
  for (const path of ["/privacy.html", "/legal.html", "/refund.html", "/cookies.html", "/feedback.html"]) {
    const response = await page.goto(path);
    expect(response?.ok()).toBeTruthy();
    await expect(page.locator("h1")).toBeVisible();
    const overflow = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
    expect(overflow).toBeLessThanOrEqual(0);
  }
});

test("compliance disclosures match current behavior", async ({ page }) => {
  await page.goto("/privacy.html");
  await expect(page.getByRole("heading", { name: "Privacy Policy" })).toBeVisible();
  await expect(page.getByText("no account system, cloud sync", { exact: false })).toBeVisible();

  await page.goto("/refund.html");
  await expect(page.getByText("does not accept payments", { exact: false })).toBeVisible();

  await page.goto("/cookies.html");
  await expect(page.getByText("itgla_admin_session", { exact: true })).toBeVisible();
  await expect(page.getByText("do not set analytics, advertising", { exact: false })).toBeVisible();
});
