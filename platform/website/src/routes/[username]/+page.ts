import type { PageLoad } from "./$types";

import { error } from "@sveltejs/kit";
import { client } from "$lib/gql";
import { graphql } from "$/gql";

/// This function will be run on SSR and on the client when the page loads.
export const load = (async ({ params }) => {
	const user = await client
		.query(
			graphql(`
				query ChannelPageUser($username: String!) {
					user: userByUsername(username: $username) {
						id
						username
						displayName
					}
				}
			`),
			{
				username: params.username,
			},
		)
		.toPromise();

	if (user.error) {
		throw error(500, {
			message: "Internal server error",
		});
	}

	if (!user.data?.user) {
		throw error(404, {
			message: "Not found",
		});
	}

	const stream = await client
		.query(
			graphql(`
				query ChannelPageStream($userId: UUID!) {
					stream: activeStreamsByUserId(id: $userId) {
						id
					}
				}
			`),
			{
				userId: user.data.user.id,
			},
		)
		.toPromise();

	if (stream.error) {
		throw error(500, {
			message: "Internal server error",
		});
	}

	return { user: user.data.user, stream: stream.data?.stream };
}) satisfies PageLoad;
