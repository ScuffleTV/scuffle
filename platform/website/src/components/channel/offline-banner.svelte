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
	// The estimated width that the banner will have on the full page.
	export let fullPageWidth: string | null = null;

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
			width="100%"
			height="100%"
			fitMode="cover"
			{fullPageWidth}
		/>
	{/if}
	<slot />
</div>

<style lang="scss">
	@import "../../assets/styles/variables.scss";

	.wrapper {
		display: inline-block;
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
