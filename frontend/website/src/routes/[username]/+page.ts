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

	return { user: user.data.user };
}) satisfies PageLoad;
