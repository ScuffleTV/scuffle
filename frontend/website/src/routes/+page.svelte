<script lang="ts">
	import ShowMore from "$/components/icons/show-more.svelte";
	import StreamPreview from "$components/stream-preview.svelte";
	import CategoryCard from "$components/category-card.svelte";

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

	const categories = [
		{
			title: "Minecraft",
			image: "/categories/minecraft.png",
			viewers: 243,
		},
		{
			title: "Valorant",
			image: "/categories/valorant.png",
			viewers: 243,
		},
		{
			title: "Deep Rock Galactic",
			image: "/categories/deep-rock-galactic.png",
			viewers: 243,
		},
		{
			title: "Fortnite",
			image: "/categories/fortnite.png",
			viewers: 243,
		},
	];
</script>

<svelte:head>
	<title>Scuffle - Home</title>
</svelte:head>

<div class="content">
	<div class="container">
		<div class="previews">
			{#each streamPreviews as preview}
				<StreamPreview {...preview} special={preview.streamer === "BTSSmash"} />
			{/each}
		</div>
	</div>

	<div class="container">
		<h2 class="title title-hbt">How about this?</h2>
		<div class="hbt">
			{#each howAboutThis as preview}
				<StreamPreview {...preview} />
			{/each}
		</div>
		<div class="show-more">
			<h4>show more</h4>
			<ShowMore />
			<hr />
		</div>
	</div>

	<div class="container">
		<h2 class="title title-categories">Categories</h2>
		<div class="categories">
			{#each categories as category}
				<CategoryCard {...category} />
			{/each}
		</div>
	</div>
</div>

<style lang="scss">
	@import "../assets/styles/variables.scss";

	.content {
		display: flex;
		flex-direction: column;
		grid-area: content;
		gap: 1rem;

		padding: 1rem;

		& > .container {
			display: flex;
			flex-direction: column;
			gap: 0.5rem;
		}
	}

	.show-more {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		cursor: pointer;
		color: #ff5a39;

		&:hover {
			color: #f47e66;
		}

		& > h4 {
			margin: 0;
			font-size: 0.95rem;
			font-weight: 400;
		}

		& > hr {
			flex-grow: 1;
			color: #252525;
		}
	}

	.previews {
		display: flex;
		flex-wrap: wrap;
		gap: 1rem;
	}

	.title-hbt,
	.title-categories {
		font-size: 1.25rem;
		font-weight: 500;
		color: #ffffff;
		font-family: "Inter", sans-serif;
		font-style: italic;
	}

	.hbt {
		display: flex;
		flex-wrap: wrap;
		gap: 1rem;
	}

	.categories {
		display: flex;
		flex-wrap: wrap;
		gap: 1rem;
	}
</style>
