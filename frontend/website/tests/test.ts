import { test } from "@playwright/test";

test("page loads", async ({ page }) => {
	await page.goto("/");
});
