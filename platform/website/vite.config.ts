import { defineConfig, searchForWorkspaceRoot } from "vite";
import { sveltekit } from "@sveltejs/kit/vite";
import { resolve } from "path";
import { execSync } from "child_process";

process.env.VITE_GIT_COMMIT = execSync("git rev-parse --short HEAD").toString().trim();
process.env.VITE_GIT_COMMIT_DATE = new Date(
	execSync("git log -1 --format=%cI").toString().trim(),
).toISOString();
process.env.VITE_GIT_BRANCH = execSync("git branch --show-current").toString().trim();
process.env.VITE_BUILD_DATE = new Date().toISOString();

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
