import { graphql } from "$/gql";
import type { User } from "$/gql/graphql";
import type { Client } from "@urql/svelte";

export async function verifyToken(client: Client, token: string): Promise<User | null> {
	const result = await client
		.mutation(
			graphql(`
				mutation LoginWithToken($token: String!) {
					auth {
						loginWithToken(sessionToken: $token, updateContext: true) {
							user {
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
							}
						}
					}
				}
			`),
			{ token },
		)
		.toPromise();

	return (result.data?.auth.loginWithToken.user as User) || null;
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
