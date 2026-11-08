const { defineConfig } = require("@playwright/test");

const port = Number.parseInt(process.env.ITGLA_TEST_PORT || "4183", 10);

module.exports = defineConfig({
  testDir: "./tests",
  outputDir: "./test-results",
  reporter: "line",
  use: {
    baseURL: process.env.ITGLA_TEST_BASE_URL || `http://127.0.0.1:${port}`,
    browserName: "chromium",
  },
  webServer: [
    {
      command: `python3 -m http.server ${port} --bind 127.0.0.1`,
      port,
      reuseExistingServer: true,
    },
    {
      command: "node tests/admin-fixture.mjs",
      port: 4174,
      reuseExistingServer: true,
    },
  ],
});
