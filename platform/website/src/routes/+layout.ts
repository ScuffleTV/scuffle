import { getUser, verifyToken } from "$/lib/auth";
import { sessionToken, user } from "$/store/auth";
import { createGqlClient } from "$/lib/gql";
import type { User } from "$/gql/graphql";

export const ssr = false;

export async function load() {
	const client = createGqlClient();
	const token = window.localStorage.getItem("auth_sessionToken");
	if (token) {
		verifyToken(client, token).then((data) => {
			sessionToken.set(data?.token || null);
		});
	} else {
		sessionToken.set(null);
	}
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
