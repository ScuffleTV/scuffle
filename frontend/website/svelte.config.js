import { vitePreprocess } from "@sveltejs/kit/vite";
import adapter from "svelte-adapter-deno";

/** @type {import('@sveltejs/kit').Config} */
const config = {
	// Consult https://kit.svelte.dev/docs/integrations#preprocessors
	// for more information about preprocessors
	preprocess: vitePreprocess(),

	kit: {
		adapter: adapter(),
		typescript: {
			config(config) {
				config.include.push("../wasm.d.ts");
			},
		},
	},
};

export default config;
