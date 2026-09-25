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
    await page.goto("http://127.0.0.1:4174/admin/");
    await expect(page.getByRole("heading", { name: "Site analytics" })).toBeVisible();

    await page.getByLabel("Username").fill("admin");
    await page.getByLabel("Password").fill("incorrect");
    await page.getByRole("button", { name: "Sign in" }).click();
    await expect(page.getByRole("alert")).toContainText("Incorrect username or password");
    errors.length = 0;

    await page.getByLabel("Password").fill("test-password");
    await page.getByRole("button", { name: "Sign in" }).click();
    await expect(page.getByRole("heading", { name: "Site overview" })).toBeVisible();
    await expect(page.getByText("1,284")).toBeVisible();
    await expect(page.locator("#trend svg")).toBeVisible();
    await expect(page.locator("#pages tbody tr")).toHaveCount(2);

    const overflow = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
    expect(overflow).toBeLessThanOrEqual(0);
    expect(errors).toEqual([]);
  });
}

test("analytics API rejects unauthenticated metrics", async ({ request }) => {
  const response = await request.get("http://127.0.0.1:4174/admin/api/overview");
  expect(response.status()).toBe(401);
});
