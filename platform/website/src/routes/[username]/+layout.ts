import { graphql } from "$/gql";
import { error } from "@sveltejs/kit";
import type { LayoutLoadEvent } from "./$types";
import { user } from "$/store/auth";
import { get } from "svelte/store";

export async function load({ params, parent }: LayoutLoadEvent) {
	const client = (await parent()).client;

	const res = await client
		.query(
			graphql(`
				query ChannelPageUser($username: String!) {
					user {
						user: byUsername(username: $username) {
							id
							username
							displayName
							displayColor {
								color
								hue
								isGray
							}
							channel {
								title
								live {
									edgeEndpoint
									organizationId
									roomId
									playerToken
									liveViewerCount
								}
								description
								followersCount
								links {
									name
									url
								}
								lastLiveAt
								category {
									name
								}
							}
						}
					}
				}
			`),
			{
				username: params.username,
			},
			{ requestPolicy: "network-only" },
		)
		.toPromise();

	if (res.error) {
		console.error(res.error);
		throw error(500, {
			message: "Internal server error",
		});
	}

	if (!res.data?.user.user) {
		throw error(404, {
			message: "Not found",
		});
	}

	let following = false;
	if (get(user)) {
		const resFollowing = await client
			.query(
				graphql(`
					query Following($channelId: ULID!) {
						user {
							following: isFollowing(channelId: $channelId)
						}
					}
				`),
				{ channelId: res.data.user.user.id },
				{ requestPolicy: "network-only" },
			)
			.toPromise();

		if (resFollowing.error || !resFollowing.data) {
			console.error(resFollowing.error);
			throw error(500, {
				message: "Internal server error",
			});
		}

		following = resFollowing.data.user.following;
	}

	const chatroomCollapsed = window.localStorage.getItem("layout_chatroomCollapsed") === "true";
	return {
		chatroomCollapsed,
		user: res.data.user.user,
		following,
	};
}
