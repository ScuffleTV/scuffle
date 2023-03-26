import { sveltekit } from "@sveltejs/kit/vite";
import { defineConfig } from "vitest/config";
// import wasm from "./plugins/wasm";
import { resolve } from "path";

export default defineConfig({
	plugins: [sveltekit()], // wasm({ directory: "../player", name: "player" })
	test: {
		include: ["src/**/*.{test,spec}.{js,ts}"],
	},
	optimizeDeps: {
		exclude: ["@urql/svelte", "urql", "@urql/core"],
	},
	resolve: {
		alias: {
			$assets: resolve(__dirname, "./src/assets"),
			$components: resolve(__dirname, "./src/components"),
		},
	},
});
