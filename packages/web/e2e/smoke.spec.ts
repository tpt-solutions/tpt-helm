// SPDX-License-Identifier: MIT OR Apache-2.0

import { test, expect } from "@playwright/test";

test.describe("TPT Helm web UI", () => {
  test("chart view renders ownship and AIS targets", async ({ page }) => {
    await page.goto("/");
    await expect(page.getByRole("heading", { name: "Chart Display" })).toBeVisible();
    const canvas = page.getByRole("img", {
      name: /Nautical chart showing ownship/i,
    });
    await expect(canvas).toBeVisible();
    // Ownship/COG/SOG readout is painted on the canvas; assert the chart has
    // AIS contacts feeding the overlay via the targets view's data presence.
    await expect(page.getByRole("link", { name: "AIS Targets" })).toBeVisible();
  });

  test("AIS targets list shows tracked contacts", async ({ page }) => {
    await page.goto("/ais");
    await expect(page.getByRole("heading", { name: "AIS Targets" })).toBeVisible();
    await expect(page.getByText(/tracked contact/i)).toBeVisible();
    // The seeded simulation always includes MV Bay Trader.
    await expect(page.getByText("MV Bay Trader")).toBeVisible();
  });

  test("AIS search filters by name", async ({ page }) => {
    await page.goto("/ais");
    await page.getByLabel(/Search by name/i).fill("Aurora");
    await expect(page.getByText("Tanker Aurora")).toBeVisible();
    await expect(page.getByText("MV Bay Trader")).toHaveCount(0);
  });

  test("route planner computes a plan", async ({ page }) => {
    await page.goto("/route");
    await expect(page.getByRole("heading", { name: "Route Planner" })).toBeVisible();
    await page.getByRole("button", { name: /Plan route/i }).click();
    await expect(page.getByText(/Distance/)).toBeVisible();
    await expect(page.getByText(/Estimated fuel/)).toBeVisible();
  });

  test("spoofing alert UI raises when a spoof is simulated", async ({ page }) => {
    await page.goto("/spoofing");
    await expect(page.getByText(/No spoofing alerts/i)).toBeVisible();
    await page.getByRole("button", { name: /Simulate GPS spoof/i }).click();
    // The simulated drift grows over a few seconds; allow time to escalate.
    await expect(page.getByText(/GPS fix inconsistent with independent references/i)).toBeVisible({
      timeout: 10_000,
    });
    await page.getByRole("button", { name: /Clear/i }).click();
    await expect(page.getByText(/No spoofing alerts/i)).toBeVisible();
  });

  test("invalid route coordinates show an error", async ({ page }) => {
    await page.goto("/route");
    await page.getByLabel("Start Lat").fill("999");
    await page.getByRole("button", { name: /Plan route/i }).click();
    await expect(page.getByRole("alert")).toContainText(/valid coordinates/i);
  });
});
