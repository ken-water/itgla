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

    await expect(page.locator("h1")).toHaveText("Know what runs where.");
    await expect(page.getByRole("link", { name: "Download for Windows", exact: true }).first()).toBeVisible();
    await expect(page.getByRole("link", { name: /All platforms & checksums/ }).first()).toBeVisible();
    await expect(page.locator(".hero-product img")).toHaveJSProperty("naturalWidth", 1422);
    await expect(page.locator(".hero-product img")).toHaveAttribute("alt", /hosting server app-01/);
    await expect(page.getByText("LOCAL INFRASTRUCTURE ATLAS · DESKTOP 0.5.0", { exact: false })).toBeVisible();
    await expect(page.getByRole("heading", { name: "Import your table" })).toBeVisible();
    await expect(page.getByRole("heading", { name: "Sort and filter" })).toBeVisible();
    await expect(page.getByRole("heading", { name: "Copy exactly" })).toBeVisible();
    await expect(page.locator(".manual-boundary")).toContainText("does not scan servers");
    await expect(page.locator(".coverage-list article")).toHaveCount(3);

    const overflow = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
    expect(overflow).toBeLessThanOrEqual(0);
    expect(consoleErrors).toEqual([]);
  });
}

test("download page exposes current verified artifacts", async ({ page, request }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto("/downloads.html");

  await expect(page.getByRole("heading", { name: "Download ITGLA v0.5.0" })).toBeVisible();
  await expect(page.locator('script[type="application/ld+json"]')).toHaveCount(1);
  await expect(page.getByRole("link", { name: "Download EXE" })).toHaveAttribute(
    "href",
    "https://itgla.com/downloads/v0.5.0/itgla-v0.5.0-windows-x86_64-setup.exe",
  );

  for (const name of ["Download ZIP", "Download DEB", "Download RPM", "Download AppImage", "Download Apple silicon DMG", "Download Intel DMG", "Download sha256sums"]) {
    await expect(page.getByRole("link", { name, exact: true })).toBeVisible();
  }
  await expect(page.getByRole("link", { name: /View full release notes/ })).toHaveAttribute(
    "href",
    "/release-notes.html",
  );
  await page.goto("/release-notes.html");
  await expect(page.getByRole("heading", { name: "Release notes" })).toBeVisible();
  await expect(page.getByText("v0.5.0 · October 24, 2026")).toBeVisible();
  await expect(page.getByText("v0.3.29 · October 18, 2026")).toBeVisible();
  await expect(page.getByText("v0.3.28 · October 18, 2026")).toBeVisible();
  await expect(page.getByText("v0.0.1 · September 19, 2026")).toBeVisible();

  const artifacts = [
    "itgla-v0.5.0-windows-x86_64-setup.exe",
    "itgla-v0.5.0-windows-x86_64.zip",
    "itgla-v0.5.0-linux-x86_64.deb",
    "itgla-v0.5.0-linux-x86_64.rpm",
    "itgla-v0.5.0-linux-x86_64.AppImage",
    "itgla-v0.5.0-macos-arm64.dmg",
    "itgla-v0.5.0-macos-x86_64.dmg",
    "sha256sums",
  ];
  for (const artifact of artifacts) {
    const response = await request.head(`${process.env.ITGLA_RELEASE_ORIGIN || ""}/downloads/v0.5.0/${artifact}`);
    expect(response.ok(), `${artifact} should be downloadable`).toBeTruthy();
    expect(Number(response.headers()["content-length"] ?? 0), `${artifact} should not be empty`).toBeGreaterThan(0);
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
    "/release-notes.html",
  ]) {
    const response = await page.goto(path);
    expect(response?.ok()).toBeTruthy();
    await expect(page.locator("h1")).toBeVisible();
    const overflow = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
    expect(overflow).toBeLessThanOrEqual(0);
  }
});

test("product content exposes structured-data and GEO entry points", async ({ page }) => {
  await page.setViewportSize({ width: 320, height: 844 });
  await page.goto("/");
  await expect(page.getByRole("heading", { name: "A direct answer before you download." })).toBeVisible();
  await expect(page.locator('script[type="application/ld+json"]')).toHaveCount(2);
  await page.goto("/import-server-inventory.html");
  await expect(page.getByRole("heading", { name: "Import a server inventory from Excel or CSV" })).toBeVisible();
  await page.goto("/server-inventory-example.html");
  await expect(page.locator("table")).toBeVisible();
  await page.locator(".policy-table").focus();
  await page.keyboard.press("ArrowRight");
  await expect.poll(() => page.locator(".policy-table").evaluate((element) => element.scrollLeft)).toBeGreaterThan(0);
  await page.goto("/");
  await page.locator(".feedback-disclosure summary").click();
  await expect(page.locator("#feedback-form")).toBeVisible();
});

test("homepage feedback supports keyboard disclosure and recovery", async ({ page }) => {
  let attempts = 0;
  await page.route("**/api/feedback", async (route) => {
    attempts += 1;
    await route.fulfill({
      status: attempts === 1 ? 503 : 200,
      contentType: "application/json",
      body: JSON.stringify(attempts === 1 ? { message: "Please try again shortly." } : { ok: true }),
    });
  });
  await page.goto("/");
  const disclosure = page.locator(".feedback-disclosure summary");
  await disclosure.focus();
  await page.keyboard.press("Enter");
  await page.getByLabel("Feedback", { exact: true }).fill("Please add a service screenshot.");
  const submit = page.getByRole("button", { name: "Send feedback" });
  await submit.click();
  await expect(page.locator("#feedback-status")).toHaveText("Please try again shortly.");
  await expect(page.getByLabel("Feedback", { exact: true })).toHaveValue("Please add a service screenshot.");
  await expect(submit).toBeEnabled();
  await submit.click();
  await expect(page.locator("#feedback-status")).toContainText("your feedback was received");
  await expect(page.getByLabel("Feedback", { exact: true })).toHaveValue("");
  expect(attempts).toBe(2);
});

test("website account remains optional along the desktop download journey", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByRole("link", { name: "Website account (optional)" })).toHaveCount(1);
  await expect(page.locator(".nav-signup")).toHaveCount(0);
  await page.getByRole("link", { name: /All platforms & checksums/ }).first().click();
  await expect(page.locator(".download-reassurance")).toContainText("No desktop account");
  await page.getByRole("link", { name: "Website account (optional)" }).click();
  await expect(page.locator(".account-boundary")).toContainText("does not sync your desktop inventory");
  await page.getByRole("link", { name: "Download the desktop app" }).click();
  await expect(page.getByRole("heading", { name: "Download ITGLA v0.5.0" })).toBeVisible();
});

test("download platform anchors and verification instructions are usable", async ({ page }) => {
  await page.setViewportSize({ width: 320, height: 800 });
  await page.goto("/downloads.html");
  await page.getByRole("link", { name: "Mac · Intel", exact: true }).click();
  await expect(page).toHaveURL(/#mac-intel$/);
  await expect(page.locator("#mac-intel")).toContainText("Unsigned and not notarized");
  await page.locator("#windows .inline-guide").click();
  await expect(page.locator("#verify-windows code").first()).toContainText("Get-FileHash");
  await expect(page.locator("#verify-linux pre")).toContainText("--ignore-missing");
  await expect(page.locator("#verify-macos pre")).toContainText("shasum -a 256");
  expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth)).toBe(0);
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
  await expect(page.locator('a[href="/auth/google/start"]')).toHaveCount(0);
  await expect(page.locator("#email-signin-form")).toHaveAttribute("id", "email-signin-form");
});

test("LLM discovery covers every sitemap page", async ({ request }) => {
  const sitemap = await request.get("/sitemap.xml");
  const llms = await request.get("/llms.txt");
  expect(sitemap.ok()).toBeTruthy();
  expect(llms.ok()).toBeTruthy();
  const sitemapPages = [...(await sitemap.text()).matchAll(/<loc>(.*?)<\/loc>/g)].map((match) => match[1]).sort();
  const llmsText = await llms.text();
  const llmsPages = [...llmsText.matchAll(/https:\/\/itgla\.com\/[^\s]*/g)].map((match) => match[0]).filter((url) => !url.endsWith("/sitemap.xml")).sort();
  expect(llmsPages).toEqual(sitemapPages);
  expect(llmsText).toContain("https://github.com/ken-water/itgla");
});

for (const viewport of viewports) {
  test(`both Mac architectures are usable at ${viewport.name}`, async ({ page }) => {
    await page.setViewportSize(viewport);
    await page.goto("/downloads.html");
    for (const [label, arch] of [["Apple silicon", "arm64"], ["Intel", "x86_64"]]) {
      await expect(page.getByRole("heading", { name: `macOS 14+ \u00b7 ${label}`, exact: true })).toBeVisible();
      await expect(page.getByRole("link", { name: `Download ${label} DMG`, exact: true })).toHaveAttribute("href", `https://itgla.com/downloads/v0.5.0/itgla-v0.5.0-macos-${arch}.dmg`);
    }
    const downloads = page.locator(".download-actions a");
    await expect(downloads).toHaveCount(7);
    const firstSize = await downloads.first().boundingBox();
    for (const link of await downloads.all()) {
      await expect(link).toHaveClass("button button-dark");
      const size = await link.boundingBox();
      expect(size.height).toBeGreaterThanOrEqual(44);
      expect(Math.abs(size.width - firstSize.width)).toBeLessThanOrEqual(1);
      expect(Math.abs(size.height - firstSize.height)).toBeLessThanOrEqual(1);
    }
    expect(await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth)).toBeLessThanOrEqual(0);
  });
}
