import { sveltekit } from "@sveltejs/kit/vite";
import { defineConfig } from "vitest/config";
import wasm from "./plugins/wasm";

export default defineConfig({
	plugins: [sveltekit(), wasm({ directory: "../player", name: "player" })],
	test: {
		include: ["src/**/*.{test,spec}.{js,ts}"],
	},
});
