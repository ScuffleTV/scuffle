import { client } from "$lib/gql";
import { get } from "svelte/store";
import { graphql } from "../gql";
import type { User } from "../gql/graphql";
import { sessionToken } from "../store/login";
import { user } from "../store/user";
import { websocketOpen } from "../store/websocket";

async function verifyToken(token: string): Promise<User | null> {
	const result = await client
		.mutation(
			graphql(`
				mutation LoginWithToken($token: String!) {
					auth {
						loginWithToken(sessionToken: $token, updateContext: true) {
							user {
								id
								username
								email
								emailVerified
								createdAt
								lastLoginAt
							}
						}
					}
				}
			`),
			{ token },
		)
		.toPromise();

	return result.data?.auth.loginWithToken.user || null;
}

async function logout(token?: string) {
	await client
		.mutation(
			graphql(`
				mutation Logout($token: String) {
					auth {
						logout(sessionToken: $token)
					}
				}
			`),
			{ token },
		)
		.toPromise();
}

export const login = (token: string, usr: User) => {
	sessionToken.set({
		token,
		source: "auth",
	});
	user.set(usr);
};

// When we mount we need to check if we have a token in local storage
// and if so, set it in the store
if (typeof window !== "undefined") {
	let oldToken = get(sessionToken);

	sessionToken.subscribe(async (token) => {
		if (token) {
			if (oldToken && oldToken.token !== token.token) {
				logout(oldToken.token);
			}

			if (token.source === "localstorage") {
				const usr = await verifyToken(token.token);
				user.set(usr);
				if (!usr) {
					sessionToken.set(null);
				}
			}
		} else if (oldToken) {
			user.set(null);
			await logout(oldToken.token);
		}

		oldToken = token;
	});

	websocketOpen.subscribe(async (open) => {
		const token = get(sessionToken);
		if (open && token) {
			const usr = await verifyToken(token.token);
			user.set(usr);
			if (!usr) {
				sessionToken.set(null);
			}
		}
	});
}
