<script lang="ts">
	import { graphql } from "$/gql";
	import { websocketOpen } from "$/store/websocket";
	import { getContextClient } from "@urql/svelte";
	import { onDestroy } from "svelte";
	import { type Subscription, subscribe, pipe } from "wonka";
	import { user } from "$/store/auth";

	let subscription: Subscription;

	const client = getContextClient();

	function sub() {
		if (!$user) return;
		subscription?.unsubscribe();
		subscription = pipe(
			client.subscription(
				graphql(`
					subscription ChannelLive($channelId: ULID!) {
						channelLive(channelId: $channelId) {
							live
						}
					}
				`),
				{ channelId: $user.id },
			),
			subscribe((res) => {
				if ($user && res.data) {
					// This is just for type safety
					// The problem here is that we want to save that the user is live but we have no more information than just that they are live
					$user.channel.live = res.data.channelLive.live
						? {
								roomId: "",
								liveViewerCount: 0,
								edgeEndpoint: "",
								organizationId: "",
							}
						: null;
				}
			}),
		);
	}

	$: if ($websocketOpen) {
		sub();
	}

	$: if (!$websocketOpen) {
		subscription?.unsubscribe();
	}

	onDestroy(() => {
		subscription?.unsubscribe();
	});
</script>

{#if $user?.channel.live}
	<a href="/creator-dashboard" class="live-indicator" title="You are live">Live</a>
{/if}

<style lang="scss">
	@import "../../assets/styles/variables.scss";

	.live-indicator {
		font-weight: 500;
		color: $textColor;
		padding: 0.5rem 1rem;
		border-radius: 0.5rem;
		background-color: $bgColor;

		text-decoration: none;

		transition: background-color 0.2s;

		&:hover {
			background-color: $bgColorLight;
		}

		&::before {
			content: "";
			display: inline-block;
			width: 0.4rem;
			height: 0.4rem;
			background-color: $liveColor;
			border-radius: 50%;
			margin-right: 0.4rem;
			margin-bottom: 0.1rem;
		}
	}
</style>
