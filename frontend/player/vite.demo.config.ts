import { defineConfig } from "vite";

export default defineConfig({
	plugins: [],
	build: {
		minify: false,
		target: "esnext",
		assetsInlineLimit: 0,
	},
});
