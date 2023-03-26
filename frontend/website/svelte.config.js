import { vitePreprocess } from "@sveltejs/kit/vite";
import adapter from "./plugins/svelte-adapter-deno/index.js";
import replace from "@rollup/plugin-replace";

/** @type {import('@sveltejs/kit').Config} */
const config = {
	// Consult https://kit.svelte.dev/docs/integrations#preprocessors
	// for more information about preprocessors
	preprocess: vitePreprocess(),

	kit: {
		adapter: adapter({
			rollupHook: (options) => {
				options.plugins.push(
					replace({
						preventAssignment: true,
						sourceMap: false,
						values: {
							"process.env.NODE_ENV": JSON.stringify("production"),
						},
					}),
				);
			},
		}),
	},
};

export default config;
