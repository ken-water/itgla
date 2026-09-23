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
    await expect(page.getByRole("heading", { name: "访问统计" })).toBeVisible();

    await page.getByLabel("用户名").fill("admin");
    await page.getByLabel("密码").fill("incorrect");
    await page.getByRole("button", { name: "登录" }).click();
    await expect(page.getByRole("alert")).toContainText("用户名或密码错误");
    errors.length = 0;

    await page.getByLabel("密码").fill("test-password");
    await page.getByRole("button", { name: "登录" }).click();
    await expect(page.getByRole("heading", { name: "站点概览" })).toBeVisible();
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
