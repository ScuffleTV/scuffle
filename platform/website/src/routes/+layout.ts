import { getUser, verifyToken } from "$/lib/auth";
import { AuthDialog, authDialog, session, user } from "$/store/auth";
import { createGqlClient } from "$/lib/gql";
import type { User } from "$/gql/graphql";

export const ssr = false;

export async function load() {
	const client = createGqlClient();
	const token = window.localStorage.getItem("auth_sessionToken");
	if (token) {
		verifyToken(client, token).then((data) => {
			if (data) {
				session.set(data);
				if (!data.twoFaSolved) {
					authDialog.set(AuthDialog.SolveTwoFa);
				}
			} else {
				session.set(null);
			}
		});
	} else {
		session.set(null);
	}
	// Save session token to localstorage when changed
	session.subscribe((session) => {
		if (session) {
			localStorage.setItem("auth_sessionToken", session.token);
			if (session.twoFaSolved) {
				// Request user
				getUser(client)
					.then((result) => {
						user.set((result.data?.user.resp as User) || null);
					})
					.catch((err) => {
						console.error("Failed to fetch user", err);
						user.set(null);
					});
			}
		} else if (session === null) {
			// Only reset session token when set to null (not undefined)
			localStorage.removeItem("auth_sessionToken");
			user.set(null);
		}
	});

	return {
		client,
	};
}
