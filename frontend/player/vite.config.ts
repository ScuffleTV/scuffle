import { defineConfig } from "vite";
import path from "path";
import dts from "vite-plugin-dts";

export default defineConfig({
	plugins: [
		dts({
			outputDir: ["dist"],
			insertTypesEntry: true,
		}),
	],
	optimizeDeps: {
		exclude: ["player-wasm"],
	},
	build: {
		minify: false,
		target: "esnext",
		lib: {
			entry: path.resolve(__dirname, "js/main.ts"),
			formats: ["es"],
			name: "Player",
			fileName: "player",
		},
		assetsInlineLimit: 0,
		rollupOptions: {
			external: [/pkg/],
		},
	},
});
