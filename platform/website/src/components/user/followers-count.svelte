<script lang="ts">
	import { graphql } from "$/gql";
	import { websocketOpen } from "$/store/websocket";
	import { getContextClient } from "@urql/svelte";
	import { onDestroy } from "svelte";
	import { type Subscription, pipe, subscribe } from "wonka";

	export let userId: string;
	export let followersCount: number;

	const client = getContextClient();

	let subscription: Subscription;

	function sub(userId: string) {
		subscription?.unsubscribe();
		subscription = pipe(
			client.subscription(
				graphql(`
					subscription FollowersCount($channelId: ULID!) {
						channelFollowersCount(channelId: $channelId)
					}
				`),
				{ channelId: userId },
			),
			subscribe((res) => {
				if (res.data) {
					followersCount = res.data.channelFollowersCount;
				}
			}),
		);
	}

	$: if ($websocketOpen) {
		sub(userId);
	}

	$: if (!$websocketOpen) {
		subscription?.unsubscribe();
	}

	onDestroy(() => {
		subscription?.unsubscribe();
	});
</script>

{followersCount}
