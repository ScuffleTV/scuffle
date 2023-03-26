import type { PlaywrightTestConfig } from "@playwright/test";

const config: PlaywrightTestConfig = {
	webServer: {
		timeout: 10 * 60 * 1000, // 10 minutes we are building WASM and it takes time to compile
		command: "pnpm build && pnpm preview",
		port: 4173,
	},
	testDir: "tests",
};

export default config;
