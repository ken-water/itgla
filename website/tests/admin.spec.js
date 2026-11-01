const { expect, test } = require("@playwright/test");

for (const viewport of [
  { name: "mobile", width: 390, height: 844 },
  { name: "desktop", width: 1440, height: 900 },
]) {
  test(`analytics admin works at ${viewport.name}`, async ({ page }) => {
    const errors = [];
    page.on("console", (message) => {
      if (message.type() === "error") errors.push(message.text());
    });
    await page.setViewportSize(viewport);
    await page.goto("http://127.0.0.1:4174/admin");
    await expect(page.getByRole("heading", { name: "Site analytics" })).toBeVisible();

    await page.getByLabel("Username").fill("admin");
    await page.getByLabel("Password").fill("incorrect");
    await page.getByRole("button", { name: "Sign in" }).click();
    await expect(page.getByRole("alert")).toContainText("Incorrect username or password");
    errors.length = 0;

    await page.getByLabel("Password").fill("test-password");
    await page.getByRole("button", { name: "Sign in" }).click();
    await expect(page.getByRole("heading", { name: "Site overview" })).toBeVisible();
    await expect(page.locator("#overview-metrics").getByText("1,284")).toBeVisible();
    await expect(page.locator("#overview-trend canvas")).toBeVisible();
    await expect(page.locator("#overview-pages tbody tr")).toHaveCount(2);
    await page.getByRole("tab", { name: "Traffic" }).click();
    await expect(page.getByRole("heading", { name: "Page views and unique visitors" })).toBeVisible();
    await expect(page.locator("#traffic-trend canvas")).toBeVisible();
    await page.getByRole("tab", { name: "Downloads" }).click();
    await expect(page.getByRole("heading", { name: "Package downloads by day" })).toBeVisible();
    await expect(page.locator("#downloads-trend canvas")).toBeVisible();
    await page.getByRole("tab", { name: "Errors" }).click();
    await expect(page.getByRole("heading", { name: "Failed requests by day" })).toBeVisible();
    await expect(page.locator("#errors-trend canvas")).toBeVisible();
    await expect(page.locator("#errors-events tbody tr")).toHaveCount(1);
    await expect(page.locator("#errors-events")).toContainText("404 Not found");
    await page.getByRole("tab", { name: "Feedback" }).click();
    await expect(page.getByRole("heading", { name: "Product feedback" })).toBeVisible();
    await expect(page.locator("#feedback-list")).toContainText("Please add a keyboard shortcut.");

    const overflow = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
    expect(overflow).toBeLessThanOrEqual(0);
    expect(errors).toEqual([]);
  });
}

test("analytics API rejects unauthenticated metrics", async ({ request }) => {
  const response = await request.get("http://127.0.0.1:4174/admin/api/overview");
  expect(response.status()).toBe(401);
});

test("slow chart and feedback resources do not block document load or sign-in", async ({ page }) => {
  await page.clock.install();
  await page.route("**/admin/chart.umd.min.js", () => {});
  await page.route("**/admin/feedback.js", () => {});
  await page.goto("http://127.0.0.1:4174/admin", { waitUntil: "load", timeout: 5000 });
  await expect(page.getByRole("button", { name: "Sign in", exact: true })).toBeEnabled();
  expect(await page.evaluate(() => document.readyState)).toBe("complete");
  expect(await page.evaluate(() => performance.getEntriesByType("resource").some((entry) => entry.name.includes("chart.umd")))).toBe(false);
  await page.getByLabel("Username").fill("admin");
  await page.getByLabel("Password").fill("test-password");
  const chartRequest = page.waitForRequest("**/admin/chart.umd.min.js");
  await page.getByRole("button", { name: "Sign in", exact: true }).click();
  await chartRequest;
  await expect(page.locator("#overview-pages tbody tr")).toHaveCount(2);
  await page.clock.fastForward(10001);
  await expect(page.locator("#overview-trend")).toContainText("Chart unavailable");
  await expect(page.locator("#dashboard-error")).toBeHidden();
  await page.getByRole("tab", { name: "Downloads", exact: true }).click();
  await expect(page.locator("#downloads-events tbody tr")).toHaveCount(1);
});

test("a stalled section preserves other data and hidden tabs are fetched on demand", async ({ page }) => {
  await page.clock.install();
  await page.route("**/admin/api/events?*", () => {});
  const paths = [];
  page.on("request", (request) => paths.push(new URL(request.url()).pathname));
  await page.goto("http://127.0.0.1:4174/admin");
  await page.getByLabel("Username").fill("admin");
  await page.getByLabel("Password").fill("test-password");
  const events = page.waitForRequest("**/admin/api/events?*");
  await page.getByRole("button", { name: "Sign in", exact: true }).click();
  await events;
  await expect(page.locator("#overview-metrics")).toContainText("1,284");
  await expect(page.locator("#overview-pages tbody tr")).toHaveCount(2);
  expect(paths).not.toContain("/admin/api/downloads");
  expect(paths).not.toContain("/admin/api/countries");
  expect(paths).not.toContain("/admin/api/download-countries");
  await page.clock.fastForward(15001);
  await expect(page.locator("#dashboard-error")).toContainText("Recent events:");
  await expect(page.locator("#overview-metrics")).toContainText("1,284");
  await page.getByRole("tab", { name: "Downloads", exact: true }).click();
  await expect(page.locator("#downloads-events tbody tr")).toHaveCount(1);
  await expect(page.locator("#dashboard-error")).toBeHidden();
});

test("stalled login request times out and restores the sign-in button", async ({ page }) => {
  await page.clock.install();
  await page.route("**/admin/api/login", () => {});
  await page.goto("http://127.0.0.1:4174/admin");
  await page.getByLabel("Username").fill("admin");
  await page.getByLabel("Password").fill("test-password");
  const request = page.waitForRequest("**/admin/api/login");
  await page.getByRole("button", { name: "Sign in", exact: true }).click();
  await request;
  await expect(page.locator("#login-button")).toHaveAttribute("aria-busy", "true");
  await page.clock.fastForward(15001);
  await expect(page.locator("#login-error")).toContainText("timed out");
  await expect(page.getByRole("button", { name: "Sign in", exact: true })).toBeEnabled();
  await expect(page.locator("#login-button")).not.toHaveAttribute("aria-busy");
  await expect(page).toHaveURL("http://127.0.0.1:4174/admin");
});

for (const failure of ["JavaScript disabled", "login script unavailable"]) {
  test(`admin cannot submit credentials with ${failure}`, async ({ browser }) => {
    const context = await browser.newContext({ javaScriptEnabled: failure !== "JavaScript disabled" });
    try {
      const page = await context.newPage();
      if (failure === "login script unavailable") {
        await page.route("**/admin/admin.js", (route) => route.abort());
      }
      await page.goto("http://127.0.0.1:4174/admin");
      await page.getByLabel("Username").fill("example-user");
      await page.getByLabel("Password").fill("not-a-real-password");
      await expect(page.locator("#login-button")).toBeDisabled();
      await expect(page.getByLabel("Username")).not.toHaveAttribute("name");
      await expect(page.getByLabel("Password")).not.toHaveAttribute("name");
      const submitted = [];
      page.on("request", (request) => {
        if (request.url().includes("/admin/api/login")) submitted.push(request);
      });
      await page.getByLabel("Password").press("Enter");
      await expect(page).toHaveURL("http://127.0.0.1:4174/admin");
      expect(submitted).toEqual([]);
    } finally {
      await context.close();
    }
  });
}

test("legacy admin credential URLs redirect to a clean sign-in page", async ({ request }) => {
  const response = await request.get("http://127.0.0.1:4174/admin?username=example&password=not-a-real-password", { maxRedirects: 0 });
  expect(response.status()).toBe(303);
  expect(response.headers().location).toBe("/admin");
  expect(response.headers()["cache-control"]).toBe("no-store");
});
