// This is a modified version of the official SvelteKit adapter for Deno
// We needed to modify it so we could add a custom rollup hook
// Currently pending a PR to the official repo
// https://github.com/pluvial/svelte-adapter-deno/issues/41
// https://github.com/pluvial/svelte-adapter-deno/pull/42
//

import { writeFileSync } from "fs";
import { rollup } from "rollup";
import { nodeResolve } from "@rollup/plugin-node-resolve";
import commonjs from "@rollup/plugin-commonjs";
import json from "@rollup/plugin-json";

import svelteDenoAdapter from "./resolve.cjs";

export default function (opts = {}) {
	const {
		out = "build",
		precompress = false,
		envPrefix = "",
		deps = `${svelteDenoAdapter}/deps.ts`,
		rollupHook = (options) => options,
	} = opts;

	return {
		name: "svelte-adapter-deno",

		async adapt(builder) {
			const tmp = builder.getBuildDirectory("deno");

			builder.rimraf(out);
			builder.rimraf(tmp);
			builder.mkdirp(tmp);

			builder.log.minor("Copying assets");
			builder.writeClient(`${out}/client${builder.config.kit.paths.base}`);
			builder.writePrerendered(`${out}/prerendered${builder.config.kit.paths.base}`);

			if (precompress) {
				builder.log.minor("Compressing assets");
				await Promise.all([
					builder.compress(`${out}/client`),
					builder.compress(`${out}/prerendered`),
				]);
			}

			builder.log.minor("Building server");

			builder.writeServer(tmp);

			writeFileSync(
				`${tmp}/manifest.js`,
				`export const manifest = ${builder.generateManifest({ relativePath: "./" })};`,
			);

			// const pkg = JSON.parse(readFileSync('package.json', 'utf8'));

			// we bundle the Vite output so that deployments only need
			// their production dependencies. Anything in devDependencies
			// will get included in the bundled code
			const options = {
				input: {
					index: `${tmp}/index.js`,
					manifest: `${tmp}/manifest.js`,
				},
				external: [
					// dependencies could have deep exports, so we need a regex
					// ...Object.keys(pkg.dependencies || {}).map((d) => new RegExp(`^${d}(\\/.*)?$`))
				],
				plugins: [
					nodeResolve({ preferBuiltins: true }),
					commonjs({
						sourceMap: false,
					}),
					json(),
				],
			};

			const bundle = await rollup(rollupHook(options) || options);

			await bundle.write({
				dir: `${out}/server`,
				format: "esm",
				sourcemap: false,
				chunkFileNames: "chunks/[name]-[hash].js",
			});

			builder.copy(`${svelteDenoAdapter}/files`, out, {
				replace: {
					ENV: "./env.js",
					HANDLER: "./handler.js",
					MANIFEST: "./server/manifest.js",
					SERVER: "./server/index.js",
					ENV_PREFIX: JSON.stringify(envPrefix),
				},
			});

			builder.log.minor(`Copying deps.ts: ${deps}`);
			builder.copy(deps, `${out}/deps.ts`);
		},
	};
}
