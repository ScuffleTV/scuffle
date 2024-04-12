// eslint.config.cjs

import eslintPluginPrettierRecommended from "eslint-plugin-prettier/recommended";
import eslintPluginSvelte from "eslint-plugin-svelte";
import js from "@eslint/js";
import svelteParser from "svelte-eslint-parser";
import tsEslint from "typescript-eslint";
import tsParser from "@typescript-eslint/parser";
import globals from "globals";

export default [
	js.configs.recommended,
	...tsEslint.configs.strict,
	...eslintPluginSvelte.configs["flat/recommended"],
	eslintPluginPrettierRecommended, // must be last to override conflicting rules.
	{
		files: ["**/*.svelte"],
		languageOptions: {
			parser: svelteParser,
			parserOptions: {
				parser: tsParser,
			},
		},
		rules: {
			"svelte/no-target-blank": "error",
			"svelte/no-at-debug-tags": "error",
			"svelte/no-reactive-functions": "error",
			"svelte/no-reactive-literals": "error",
		},
	},
	{
		languageOptions: {
			globals: {
				...globals.browser,
				...globals.node,
				...globals.es2020,
				NodeJS: true,
			},
		},
	},
	{
		ignores: ["src/gql/*.ts"],
	},
];
