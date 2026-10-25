import { expect, test } from "@playwright/test";

test("health endpoint answers", async ({ request }) => {
  const response = await request.get("/health");
  expect(response.ok()).toBe(true);
  expect(await response.json()).toMatchObject({ status: "ok" });
});

test("rpc answers Ping", async ({ request }) => {
  const response = await request.post("/api/rpc", { data: { type: "Ping" } });
  expect(response.ok()).toBe(true);
  expect(await response.json()).toEqual({ type: "Pong" });
});

test("library screen loads and the event stream connects", async ({ page }) => {
  const events = page.waitForEvent("websocket", (socket) => socket.url().endsWith("/api/events"));
  await page.goto("/");
  await expect(page.getByRole("heading", { level: 1, name: "Library" })).toBeVisible();
  await expect(page.getByText("No library open")).toBeVisible();
  const socket = await events;
  expect(socket.isClosed()).toBe(false);
  // The link to the core shows only when it drops (plan task 8).
  await expect(page.getByTestId("connection-lost")).toHaveCount(0);
  await expect(page.getByText("Connected")).toHaveCount(0);
});

test("settings show the core version over HTTP", async ({ page }) => {
  await page.goto("/settings");
  await expect(page.getByTestId("app-version")).toHaveText(/^\d+\.\d+\.\d+/);
  await expect(page.getByText("Server", { exact: true })).toBeVisible();
});

test("deep links are served by the SPA", async ({ page }) => {
  await page.goto("/jobs");
  await expect(page.getByRole("heading", { level: 1, name: "Jobs" })).toBeVisible();
});

test("theme persists across reloads", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByRole("heading", { level: 1, name: "Library" })).toBeVisible();
  await page.getByRole("button", { name: "Switch to dark theme" }).click();
  await expect(page.locator("html")).toHaveClass(/\bdark\b/);

  await page.reload();
  await expect(page.getByRole("heading", { level: 1, name: "Library" })).toBeVisible();
  await expect(page.locator("html")).toHaveClass(/\bdark\b/);
  await expect(page.locator("html")).toHaveAttribute("lang", "en");
});
