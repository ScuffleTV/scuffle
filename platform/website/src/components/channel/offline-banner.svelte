<script lang="ts">
	import { graphql } from "$/gql";
	import { pipe, subscribe, type Subscription } from "wonka";
	import { type ImageUpload } from "$/gql/graphql";
	import { getContextClient } from "@urql/svelte";
	import { onDestroy } from "svelte";
	import { websocketOpen } from "$/store/websocket";
	import ResponsiveImage from "../responsive-image.svelte";

	export let channelId: string;
	export let offlineBanner: ImageUpload | null | undefined;

	const client = getContextClient();

	let sub: Subscription;

	function subToOfflineBanner(channelId: string) {
		sub?.unsubscribe();
		sub = pipe(
			client.subscription(
				graphql(`
					subscription OfflineBanner($channelId: ULID!) {
						channelOfflineBanner(channelId: $channelId) {
							offlineBanner {
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
				{ channelId },
			),
			subscribe(({ data }) => {
				if (data) {
					offlineBanner = data.channelOfflineBanner.offlineBanner;
				}
			}),
		);
	}

	$: if ($websocketOpen) {
		subToOfflineBanner(channelId);
	}

	$: if (!$websocketOpen) {
		sub?.unsubscribe();
	}

	onDestroy(() => {
		sub?.unsubscribe();
	});
</script>

<div class="wrapper" class:has-banner={offlineBanner}>
	{#if offlineBanner}
		<ResponsiveImage
			image={offlineBanner}
			alt="offline banner"
			background
			aspectRatio="5/1"
			width="100%"
			height="100%"
			fitMode="cover"
		/>
	{/if}
	<slot />
</div>

<style lang="scss">
	@import "../../assets/styles/variables.scss";

	.wrapper {
		display: inline-block;
		overflow: hidden;
		position: relative;

		width: 100%;
		background-color: $bgColorLight;
		z-index: 0;

		display: flex;
		align-items: center;

		&.has-banner {
			aspect-ratio: 5 / 1;
		}
	}
</style>
