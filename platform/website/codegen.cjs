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
				strictScalars: true,
				scalars: {
					DateRFC3339: "string",
					UUID: "string",
					ULID: "string",
					Color: "string",
				},
			},
		},
	},
};

module.exports = config;
