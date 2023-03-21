import type { Adapter } from "@sveltejs/kit";
import type { RollupOptions } from "rollup";

interface AdapterOptions {
	out?: string;
	precompress?: boolean;
	envPrefix?: string;
	deps?: string;
	rollupHook?: (options: RollupOptions) => RollupOptions | void;
}

export default function plugin(options?: AdapterOptions): Adapter;
