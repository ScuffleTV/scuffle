import { error } from "@sveltejs/kit";
import type { PageLoadEvent } from "./$types";

export async function load({ url }: PageLoadEvent) {
	const query = url.searchParams.get("q");

	if (!query) {
		throw error(404, "Not found");
	}

	return { query };
}
