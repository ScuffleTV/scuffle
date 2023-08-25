import { verifyToken } from "$/lib/auth";
import { sessionToken, user } from "$/store/auth";
import { createGqlClient } from "$/lib/gql";
import { get } from "svelte/store";

export const ssr = false;

export async function load() {
	const token = get(sessionToken);
	const client = createGqlClient();
	user.set(token ? await verifyToken(client, token) : null);

	return {
		client,
	};
}
