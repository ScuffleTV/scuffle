import type { PlaywrightTestConfig } from "@playwright/test";

const config: PlaywrightTestConfig = {
	webServer: {
		command: "yarn build && yarn preview",
		port: 4173,
	},
	testDir: "tests",
};

export default config;
