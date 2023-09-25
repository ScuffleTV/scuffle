module.exports = {
	root: true,
	parser: "@typescript-eslint/parser",
	extends: [
		"eslint:recommended",
		"plugin:@typescript-eslint/recommended",
		"prettier",
		"plugin:svelte/recommended",
	],
	plugins: ["@typescript-eslint"],
	ignorePatterns: ["*.cjs", "src/gql/*"],
	overrides: [
		{
			files: ["*.svelte"],
			parser: "svelte-eslint-parser",
			parserOptions: {
				parser: {
					ts: "@typescript-eslint/parser",
					js: "espree",
					typescript: "@typescript-eslint/parser",
				},
			},
		},
	],
	globals: {
		NodeJS: true,
	},
	rules: {
		"no-unused-vars": "off",
		"@typescript-eslint/no-unused-vars": [
			"error",
			{
				argsIgnorePattern: "^_",
				varsIgnorePattern: "^_",
				caughtErrorsIgnorePattern: "^_",
			},
		],
	},
	parserOptions: {
		sourceType: "module",
		ecmaVersion: 2024,
		extraFileExtensions: [".svelte"],
	},
	env: {
		browser: true,
		es2017: true,
		node: true,
	},
};
