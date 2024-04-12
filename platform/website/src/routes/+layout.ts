import { getUser } from "$/lib/auth";
import { sessionToken, user } from "$/store/auth";
import { createGqlClient } from "$/lib/gql";
import type { User } from "$/gql/graphql";

export const ssr = false;

export async function load() {
	sessionToken.set(window.localStorage.getItem("auth_sessionToken"));
	const client = createGqlClient();
	// Save session token to localstorage when changed
	sessionToken.subscribe((token) => {
		if (token) {
			localStorage.setItem("auth_sessionToken", token);
			// Request user
			getUser(client)
				.then((result) => {
					user.set((result.data?.user.resp as User) || null);
				})
				.catch((err) => {
					console.error("Failed to fetch user", err);
					user.set(null);
				});
		} else if (token === null) {
			// Only reset session token when set to null (not undefined)
			localStorage.removeItem("auth_sessionToken");
			user.set(null);
		}
	});

	return {
		client,
	};
}
