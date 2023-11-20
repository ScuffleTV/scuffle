<script lang="ts">
	import SmallStreamPreview from "$/components/home/small-stream-preview.svelte";
	import type { User } from "$/gql/graphql";
	import { PUBLIC_ASSET_BASE_URL } from "$env/static/public";

	const following = [
		{
			user: {
				id: "01H8WMQ6EPH7YFM1PQTJF81TT7",
				username: "troykomodo",
				displayName: "TroyKomodo",
				channel: {
					title: "working on https://github.com/ScuffleTV/scuffle",
					liveViewerCount: 1,
				},
			},
			avatar:
				"https://static-cdn.jtvnw.net/jtv_user_pictures/3773bfdd-110b-4911-b914-6f04362a1331-profile_image-70x70.png",
			preview: "/troykomodo-preview.png",
		},
	] as {
		user: User;
		avatar: string;
		preview: string;
	}[];
</script>

<svelte:head>
	<title>Scuffle - Following</title>

	<!-- Open Graph -->
	<meta property="og:title" content="Scuffle - Following" />
	<meta property="og:description" content="Scuffle - open-source live-streaming platform" />
	<meta property="og:image" content="{PUBLIC_ASSET_BASE_URL}/banner.jpeg" />
	<meta property="og:image:alt" content="Scuffle Banner" />
</svelte:head>

<div class="content" aria-label="Page content">
	<h2 class="title" id="following-title">Following</h2>
	<section class="following" aria-labelledby="following-title">
		{#each following as preview}
			<article>
				<SmallStreamPreview user={preview.user} avatar={preview.avatar} preview={preview.preview} />
			</article>
		{/each}
	</section>
</div>

<style lang="scss">
	@import "../../assets/styles/variables.scss";

	.content {
		grid-area: content;
		overflow-y: auto;

		padding: 1rem;
	}

	.title {
		font-size: 1.25rem;
		font-weight: 500;
		color: $textColor;
		font-family: $sansFont;

		margin-bottom: 0.5rem;
	}

	.following {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(18rem, 1fr));
		gap: 2rem;
	}
</style>
