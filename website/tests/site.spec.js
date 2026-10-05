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
    await expect(page.getByRole("link", { name: "Download Windows EXE" }).first()).toBeVisible();
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
  await expect(page.locator('script[type="application/ld+json"]')).toHaveCount(1);
  await expect(page.getByRole("link", { name: "Download EXE" })).toHaveAttribute(
    "href",
    "/downloads/itgla-v0.2.2-windows-x86_64-setup.exe",
  );

  for (const name of ["Download ZIP", "Download DEB", "Download RPM", "Download AppImage", "Download DMG", "Download SHA256SUMS"]) {
    await expect(page.getByRole("link", { name, exact: true })).toBeVisible();
  }

  const overflow = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
  expect(overflow).toBeLessThanOrEqual(0);
});

test("secondary documents render", async ({ page }) => {
  for (const path of [
    "/privacy.html",
    "/legal.html",
    "/refund.html",
    "/cookies.html",
    "/server-inventory.html",
    "/import-server-inventory.html",
    "/custom-server-fields.html",
    "/server-spreadsheet-alternative.html",
    "/local-first-server-inventory.html",
    "/server-inventory-example.html",
  ]) {
    const response = await page.goto(path);
    expect(response?.ok()).toBeTruthy();
    await expect(page.locator("h1")).toBeVisible();
    const overflow = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
    expect(overflow).toBeLessThanOrEqual(0);
  }
});

test("product content exposes structured-data and GEO entry points", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByRole("heading", { name: "A direct answer before you download." })).toBeVisible();
  await expect(page.locator('script[type="application/ld+json"]')).toHaveCount(2);
  await page.goto("/import-server-inventory.html");
  await expect(page.getByRole("heading", { name: "Import a server inventory from Excel or CSV" })).toBeVisible();
  await page.goto("/server-inventory-example.html");
  await expect(page.locator("table")).toBeVisible();
  await page.goto("/");
  await expect(page.locator("#feedback-form")).toBeVisible();
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

test("email sign-in does not fall through to a 404", async ({ page }) => {
  await page.goto("/signin.html");
  await expect(page.locator("#email-signin-form")).toBeVisible();
  await expect(page.locator("#email-signin-form")).toHaveAttribute("id", "email-signin-form");
});
