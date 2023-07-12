import { vitePreprocess } from "@sveltejs/kit/vite";
import adapter from "@sveltejs/adapter-node";

/** @type {import('@sveltejs/kit').Config} */
const config = {
	preprocess: vitePreprocess(),

	kit: {
		adapter: adapter({
			envPrefix: "SCUF_",
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
	},
};

export default config;
