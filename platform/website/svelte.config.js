import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";
import adapter from "@sveltejs/adapter-static";

/** @type {import('@sveltejs/kit').Config} */
const config = {
	preprocess: vitePreprocess(),

	kit: {
		adapter: adapter({
			envPrefix: "SCUF_",
			pages: "dist",
			assets: "dist",
			fallback: "index.html",
		}),
		alias: {
			$: "./src",
			$lib: "./src/lib",
			$components: "./src/components",
			$icons: "./src/components/icons",
			$store: "./src/store",
			$gql: "./src/gql",
		},
		typescript: {
			config(cfg) {
				cfg.compilerOptions.ignoreDeprecations = "5.0";
				return cfg;
			},
		},
		prerender: {
			handleHttpError: "warn",
		},
	},
};

export default config;
