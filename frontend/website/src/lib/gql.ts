import { createClient } from "@urql/svelte";
import { sessionToken } from "../store/login";
import { get } from "svelte/store";

// This GQL context is created once and is available to all components.
export const client = createClient({
	url: import.meta.env.VITE_GQL_ENDPOINT,
	fetchOptions: () => {
		const token = get(sessionToken);
		return {
			headers: token
				? {
						authorization: `Bearer ${token}`,
				  }
				: undefined,
		};
	},
});
