import { defineConfig } from "vite";

export default defineConfig({
	plugins: [],
	build: {
		minify: false,
		target: "esnext",
		outDir: "demo-dist",
		assetsInlineLimit: 0,
	},
});
