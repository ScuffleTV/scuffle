module.exports = {
	root: true,
	parser: "@typescript-eslint/parser",
	extends: ["eslint:recommended", "plugin:@typescript-eslint/recommended", "prettier"],
	plugins: ["@typescript-eslint"],
	ignorePatterns: ["*.cjs", "src/gql/*"],
	globals: {
		NodeJS: true,
	},
	parserOptions: {
		sourceType: "module",
		ecmaVersion: 2020,
	},
	env: {
		browser: true,
		es2017: true,
		node: true,
	},
};
