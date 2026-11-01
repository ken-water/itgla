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
    await expect(page.locator("#overview-trend > svg")).toBeVisible();
    await expect(page.locator("#overview-pages tbody tr")).toHaveCount(2);
    await page.getByRole("tab", { name: "Traffic" }).click();
    await expect(page.getByRole("heading", { name: "Page views and unique visitors" })).toBeVisible();
    await expect(page.locator("#traffic-trend > svg")).toBeVisible();
    await page.getByRole("tab", { name: "Downloads" }).click();
    await expect(page.getByRole("heading", { name: "Package downloads by day" })).toBeVisible();
    await expect(page.locator("#downloads-trend > svg")).toBeVisible();
    await page.getByRole("tab", { name: "Errors" }).click();
    await expect(page.getByRole("heading", { name: "Failed requests by day" })).toBeVisible();
    await expect(page.locator("#errors-trend > svg")).toBeVisible();
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
  for (const name of ["overview", "download-summary", "download-events"]) {
    const response = await request.get(`http://127.0.0.1:4174/admin/api/${name}`);
    expect(response.status()).toBe(401);
  }
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
  await page.getByRole("button", { name: "Sign in", exact: true }).click();
  await expect(page.locator("#overview-pages tbody tr")).toHaveCount(2);
  await page.clock.fastForward(60001);
  await expect(page.locator("#overview-trend > svg")).toBeVisible();
  expect(await page.evaluate(() => performance.getEntriesByType("resource").some((entry) => entry.name.includes("chart.umd")))).toBe(false);
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
  const retry = page.waitForRequest("**/admin/api/events?*");
  await page.clock.fastForward(15001);
  await retry;
  await page.clock.fastForward(15001);
  await expect(page.locator("#dashboard-error")).toContainText("Recent events:");
  await expect(page.locator("#overview-metrics")).toContainText("1,284");
  await page.getByRole("tab", { name: "Downloads", exact: true }).click();
  await expect(page.locator("#downloads-events tbody tr")).toHaveCount(1);
  await expect(page.locator("#dashboard-error")).toBeHidden();
});

async function signIn(page) {
  await page.goto("http://127.0.0.1:4174/admin");
  await page.getByLabel("Username").fill("admin");
  await page.getByLabel("Password").fill("test-password");
  await page.getByRole("button", { name: "Sign in", exact: true }).click();
  await expect(page.locator("#overview-trend > svg")).toBeVisible();
}

test("native charts contain real series and accessible exact values", async ({ page }) => {
  await signIn(page);
  await page.getByRole("tab", { name: "Traffic", exact: true }).click();
  await expect(page.locator("#traffic-trend .chart-series")).toHaveCount(2);
  await expect(page.locator("#country-trend > svg")).toBeVisible();
  await page.locator("#traffic-trend summary").click();
  await expect(page.locator("#traffic-trend table tbody tr")).toHaveCount(3);
  await expect(page.locator("#traffic-trend table")).toContainText("Page views");
});

test("valid session survives reload with browser cache disabled", async ({ page, context }) => {
  await signIn(page);
  const cdp = await context.newCDPSession(page);
  await cdp.send("Network.enable");
  await cdp.send("Network.setCacheDisabled", { cacheDisabled: true });
  for (let attempt = 0; attempt < 2; attempt++) {
    await page.reload();
    await expect(page.locator("#overview-trend > svg")).toBeVisible();
    await expect(page.locator("#login")).toBeHidden();
  }
  await page.getByRole("button", { name: "Sign out", exact: true }).click();
  await expect(page.locator("#login")).toBeVisible();
  await page.reload();
  await expect(page.locator("#login")).toBeVisible();
});

test("session check timeout offers retry without asking for credentials", async ({ page }) => {
  await signIn(page);
  await page.clock.install();
  await page.route("**/admin/api/me", () => {});
  const pending = page.waitForRequest("**/admin/api/me");
  await page.reload();
  await pending;
  await expect(page.locator("#login")).toBeHidden();
  await page.clock.fastForward(15001);
  await expect(page.locator("#session-message")).toContainText("session has not been cleared");
  await expect(page.locator("#login")).toBeHidden();
  await page.unroute("**/admin/api/me");
  await page.getByRole("button", { name: "Retry session check" }).click();
  await expect(page.locator("#overview-trend > svg")).toBeVisible();
});

test("expired session requires sign-in and sign-out failure stays visible", async ({ page, context }) => {
  await signIn(page);
  await page.route("**/admin/api/logout", (route) => route.abort());
  await page.getByRole("button", { name: "Sign out", exact: true }).click();
  await expect(page.locator("#dashboard-error")).toContainText("Sign out failed");
  await expect(page.locator("#login")).toBeHidden();
  await context.clearCookies();
  await page.reload();
  await expect(page.locator("#login")).toBeVisible();
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
      await expect(page.locator("#login")).toBeHidden();
      await expect(page.locator("#login-button")).toBeDisabled();
      await expect(page.locator("#username")).not.toHaveAttribute("name");
      await expect(page.locator("#password")).not.toHaveAttribute("name");
      const submitted = [];
      page.on("request", (request) => {
        if (request.url().includes("/admin/api/login")) submitted.push(request);
      });
      await page.evaluate(() => {
        document.querySelector("#username").value = "example-user";
        document.querySelector("#password").value = "not-a-real-password";
        document.querySelector("#password").dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true }));
      });
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
