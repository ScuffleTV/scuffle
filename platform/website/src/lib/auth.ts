import { graphql } from "$/gql";
import type { Client } from "@urql/svelte";

export async function verifyToken(client: Client, token: string) {
	const result = await client
		.mutation(
			graphql(`
				mutation LoginWithToken($token: String!) {
					auth {
						loginWithToken(sessionToken: $token, updateContext: true) {
							token
							twoFaSolved
						}
					}
				}
			`),
			{ token },
		)
		.toPromise();

	return result.data?.auth.loginWithToken || null;
}

export function getUser(client: Client) {
	return client
		.query(
			graphql(`
				query GetUser {
					user {
						resp: withCurrentContext {
							id
							displayName
							displayColor {
								color
								hue
								isGray
							}
							username
							email
							emailVerified
							lastLoginAt
							channel {
								id
								liveViewerCount
							}
							totpEnabled
						}
					}
				}
			`),
			{},
		)
		.toPromise();
}

export async function logout(client: Client, token?: string) {
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
