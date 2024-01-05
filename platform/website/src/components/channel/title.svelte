<script lang="ts">
	import { graphql } from "$/gql";
	import { websocketOpen } from "$/store/websocket";
	import { getContextClient } from "@urql/svelte";
	import { onDestroy } from "svelte";
	import { type Subscription, subscribe, pipe } from "wonka";

	export let channelId: string;
	export let title: string | null | undefined;

	let subscription: Subscription;

	const client = getContextClient();

	function sub(channelId: string) {
		subscription?.unsubscribe();
		subscription = pipe(
			client.subscription(
				graphql(`
					subscription ChannelTitle($channelId: ULID!) {
						channelTitle(channelId: $channelId) {
							title
						}
					}
				`),
				{ channelId },
			),
			subscribe((res) => {
				if (res.data) {
					title = res.data.channelTitle.title;
				}
			}),
		);
	}

	$: if ($websocketOpen) {
		sub(channelId);
	}

	$: if (!$websocketOpen) {
		subscription?.unsubscribe();
	}

	onDestroy(() => {
		subscription?.unsubscribe();
	});
</script>

{title}
