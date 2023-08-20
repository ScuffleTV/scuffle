import { defineConfig, searchForWorkspaceRoot } from "vite";
import { sveltekit } from "@sveltejs/kit/vite";
import { resolve } from "path";

export default defineConfig({
	plugins: [sveltekit()],
	optimizeDeps: {
		exclude: ["@urql/svelte", "urql", "@urql/core", "@scuffle/player"],
	},
	server: {
		fs: {
			allow: [searchForWorkspaceRoot(__dirname)],
		},
	},
	ssr: {
		noExternal: true,
	},
	resolve: {
		alias: {
			$: resolve(__dirname, "./src"),
			$assets: resolve(__dirname, "./src/assets"),
			$components: resolve(__dirname, "./src/components"),
		},
	},
	build: {
		commonjsOptions: {
			include: [/node_modules/],
		},
	},
});
