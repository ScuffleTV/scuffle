import { createClient } from "@urql/svelte";

// This GQL context is created once and is available to all components.
export const client = createClient({
	url: import.meta.env.VITE_GQL_ENDPOINT,
});
