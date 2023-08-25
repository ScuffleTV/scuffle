import { error } from "@sveltejs/kit";

export function load({ url }) {
	const query = url.searchParams.get("q");

	if (!query) {
		throw error(404, "Not found");
	}

	return { query };
}
