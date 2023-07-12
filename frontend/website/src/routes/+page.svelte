<script lang="ts">
	import StreamPreview from "$components/stream-preview.svelte";

	const streamPreviews = [
		{
			streamer: "BTSSmash",
			title: "PotFriend Fan Club | !twitter !pobox !youtube",
			tags: ["Just Chatting", "English", "reacts", "memes"],
			viewers: 1100,
			avatar:
				"https://static-cdn.jtvnw.net/jtv_user_pictures/nymn-profile_image-d52821b50793580f-300x300.jpeg",
			preview: "/nymn-preview.png",
		},
		{
			streamer: "realSport",
			title: "REAL SPORTS WITH BRYANT GUMBEL",
			tags: ["Sport", "Chinese", "Hockey", "Baseball"],
			viewers: 1_200_000,
			avatar:
				"https://static-cdn.jtvnw.net/jtv_user_pictures/4a1b51a9-0094-4cb8-bdf4-dba7c7e64dea-profile_image-300x300.png",
			preview: "/realsport-preview.png",
		},
		{
			streamer: "xQc",
			title: "LIVE👏CLICK👏NOW👏DRAMA👏NEWS👏PAGMAN",
			tags: ["Minecraft", "LGBTQ+", "NoDamage"],
			viewers: 75300,
			avatar:
				"https://static-cdn.jtvnw.net/jtv_user_pictures/xqc-profile_image-9298dca608632101-300x300.jpeg",
			preview: "/xqc-preview.png",
		},
		{
			streamer: "rainbolt",
			title: "GEOGUESSER WITH CHAT",
			tags: ["Geoguesser", "English", "interact", "bigbrain"],
			viewers: 100,
			avatar:
				"https://static-cdn.jtvnw.net/jtv_user_pictures/1310788d-3ea3-488a-8d77-5585a97dfe90-profile_image-300x300.png",
			preview: "/rainbolt-preview.png",
		},
	];

	const howAboutThis = [
		...streamPreviews,
		{
			streamer: "xQc",
			title: "LIVE👏CLICK👏NOW👏DRAMA👏NEWS👏PAGMAN",
			tags: ["Minecraft", "LGBTQ+", "NoDamage"],
			viewers: 75300,
			avatar:
				"https://static-cdn.jtvnw.net/jtv_user_pictures/xqc-profile_image-9298dca608632101-300x300.jpeg",
			preview: "/xqc-preview.png",
		},
	];
</script>

<svelte:head>
	<title>Scuffle - Home</title>
</svelte:head>

<div class="content">
	<div class="previews">
		{#each streamPreviews as preview}
			<StreamPreview {...preview} special={preview.streamer === "BTSSmash"} />
		{/each}
	</div>

	<h2 class="title title-hbt">How about this?</h2>

	<div class="hbt">
		{#each howAboutThis as preview}
			<StreamPreview {...preview} />
		{/each}
	</div>

	<h2 class="title title-categories">Categories</h2>

	<div class="categories" />
</div>

<style lang="scss">
	@import "../assets/styles/variables.scss";

	.content {
		display: flex;
		flex-direction: column;
		grid-area: content;
	}

	.previews {
		padding: 1rem;
		display: flex;
		flex-wrap: wrap;
		column-gap: 1rem;
		row-gap: 1rem;
		grid-area: previews;
	}

	.title-hbt {
		grid-area: hbt-title;
	}

	.title-categories {
		grid-area: categories-title;
	}

	.title-hbt,
	.title-categories {
		padding-left: 1rem;
		font-size: 1.25rem;
		font-weight: 500;
		color: #ffffff;
		font-family: "Inter", sans-serif;
		font-style: italic;
	}

	.hbt {
		padding: 1rem;
		display: flex;
		flex-wrap: wrap;
		column-gap: 1rem;
		row-gap: 1rem;
		grid-area: hbt;
	}

	.categories {
		padding: 1rem;
		display: flex;
		flex-wrap: wrap;
		column-gap: 1rem;
		row-gap: 1rem;
		grid-area: categories;
	}
</style>
