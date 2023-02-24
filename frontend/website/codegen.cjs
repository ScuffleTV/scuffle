/** @type {import('@graphql-codegen/cli').CodegenConfig} */
const config = {
	overwrite: true,
	schema: process.env.SCHEMA_URL || "./schema.graphql",
	documents: ["./src/**/*.svelte", "./src/**/*.graphql", "./src/**/*.ts"],
	generates: {
		"src/gql/": {
			preset: "client",
			config: {
				useTypeImports: true,
			},
		},
	},
};

module.exports = config;
