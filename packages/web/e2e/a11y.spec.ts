// SPDX-License-Identifier: MIT OR Apache-2.0

import { test, expect } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";

// Accessibility (WCAG 2.1 AA) smoke checks across the main views. These run in
// Chromium only; cross-browser visual/behavior coverage is in smoke.spec.ts.
test.describe("accessibility", () => {
  const routes = [
    { path: "/", name: "Chart Display" },
    { path: "/ais", name: "AIS Targets" },
    { path: "/route", name: "Route Planner" },
    { path: "/spoofing", name: "Spoofing Alerts" },
  ];

  for (const route of routes) {
    test(`${route.path} has no detectable violations`, async ({ page }) => {
      await page.goto(route.path);
      await expect(
        page.getByRole("heading", { name: route.name }),
      ).toBeVisible();
      const results = await new AxeBuilder({ page })
        .withTags(["wcag2a", "wcag2aa"])
        .analyze();
      expect(results.violations).toEqual([]);
    });
  }
});
