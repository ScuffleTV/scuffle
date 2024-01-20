<script lang="ts">
	import { graphql } from "$/gql";
	import { pipe, subscribe, type Subscription } from "wonka";
	import { type DisplayColor, type ImageUpload } from "$/gql/graphql";
	import { getContextClient } from "@urql/svelte";
	import { onDestroy } from "svelte";
	import { websocketOpen } from "$/store/websocket";
	import DefaultAvatar from "./default-avatar.svelte";
	import ResponsiveImage from "../responsive-image.svelte";

	export let userId: string;
	export let profilePicture: ImageUpload | null | undefined;
	export let displayColor: DisplayColor;
	export let size: number = 48;

	const client = getContextClient();

	let sub: Subscription;

	function subToProfilePicture(userId: string) {
		sub?.unsubscribe();
		sub = pipe(
			client.subscription(
				graphql(`
					subscription ProfilePicture($userId: ULID!) {
						userProfilePicture(userId: $userId) {
							profilePicture {
								id
								variants {
									width
									height
									scale
									url
									format
									byteSize
								}
								endpoint
							}
						}
					}
				`),
				{ userId },
			),
			subscribe(({ data }) => {
				if (data) {
					profilePicture = data.userProfilePicture.profilePicture;
				}
			}),
		);
	}

	$: if ($websocketOpen) {
		subToProfilePicture(userId);
	}

	$: if (!$websocketOpen) {
		sub?.unsubscribe();
	}

	onDestroy(() => {
		sub?.unsubscribe();
	});
</script>

{#if profilePicture}
	<ResponsiveImage variants={profilePicture.variants} endpoint={profilePicture.endpoint} {size} alt="avatar" rounded />
{:else}
	<DefaultAvatar {userId} {displayColor} {size} />
{/if}
