import fg from "fast-glob";
import fs from "fs-extra";
import type { Plugin } from "vite";
import { createHash } from "crypto";
import { resolve, basename } from "path";

interface WasmPluginOptions {
	name: string;
	directory: string;
}

async function get_files(directory: string) {
	return await fg([`${directory}/**/*`]);
}

const PREFIX = "@wasm@";

const typescriptModules = new Map<string, string>();

async function update_module_ts(directory: string, name: string) {
	const file = await fs.readFile(`${directory}/pkg/${name}.d.ts`, "utf-8");
	typescriptModules.set(name, file);

	const modules = [...typescriptModules.entries()]
		.map(([name, module]) => `declare module "${name}" {\n${module}\n}`)
		.join("\n");

	await fs.writeFile(`./wasm.d.ts`, modules);
}

export default function wasmPlugin(options: WasmPluginOptions): Plugin {
	// Get absolute path to the directory
	options.directory = resolve(options.directory);

	const moduleId = options.name;

	let loaded = false;

	let wasmFile = fs.readFileSync(`${options.directory}/pkg/${options.name}_bg.wasm`);

	let isDev = false;

	return {
		name: "wasm",

		// This is run on both dev and build mode
		apply(_, env) {
			if (env.mode === "development") {
				isDev = true;
			}

			return true;
		},

		// This is run on both dev and build mode
		resolveId(id) {
			if (id !== moduleId) {
				return;
			}

			return PREFIX + id;
		},

		// This is run on both dev and build mode
		async load(id) {
			if (id !== PREFIX + moduleId) {
				return;
			}

			loaded = true;

			const hash = createHash("sha256").update(wasmFile).digest("hex").slice(0, 8);
			return await fs
				.readFile(`${options.directory}/pkg/${options.name}.js`, "utf-8")
				.then((data) =>
					data.replaceAll(
						`player_bg.wasm`,
						isDev ? `${options.name}_bg.wasm` : `assets/${options.name}_${hash}_bg.wasm`,
					),
				);
		},

		// This is run on both dev and build mode
		async buildStart() {
			const files = await get_files(`${options.directory}/pkg`);
			for (const file of files) {
				this.addWatchFile(file);
			}

			await update_module_ts(options.directory, options.name);
		},

		// This is only run in dev mode
		async handleHotUpdate({ file, server }) {
			const module = server.moduleGraph.getModuleById(PREFIX + options.name);

			if (!file.startsWith(options.directory) || !module) {
				return;
			}

			wasmFile = await fs.readFile(`${options.directory}/pkg/${options.name}_bg.wasm`);
			await update_module_ts(options.directory, options.name);

			loaded = false;

			await server.reloadModule(module);
		},

		// This is only run in dev mode
		configureServer({ middlewares }) {
			middlewares.use((req, res, next) => {
				if (!req.url) {
					next();
					return;
				}

				const file = basename(req.url);
				if (file === `${options.name}_bg.wasm`) {
					res.setHeader("Cache-Control", "no-cache, no-store, must-revalidate");

					res.writeHead(200, { "Content-Type": "application/wasm" });
					res.write(wasmFile);
					res.end();
				} else {
					next();
				}
			});
		},

		// This is only run in build mode
		buildEnd() {
			if (!loaded) {
				return;
			}

			const hash = isDev
				? ""
				: "_" + createHash("sha256").update(wasmFile).digest("hex").slice(0, 8);

			this.emitFile({
				type: "asset",
				fileName: `assets/${options.name}${hash}_bg.wasm`,
				source: wasmFile,
			});
		},
	};
}
