<script lang="ts">
	import Fa from "svelte-fa";
	import { faChevronDown } from "@fortawesome/free-solid-svg-icons";
	import CategoryCard from "$/components/home/category-card.svelte";
	import SmallStreamPreview from "$/components/home/small-stream-preview.svelte";
	import BigStreamPreview from "$/components/home/big-stream-preview.svelte";

	// We should always load 13 previews because that means that we show 12 previews in the "How about this?" section
	// 12 is a nice number because it's divisible by 2, 3, 4, and 6 which means that it fills all rows in the grid layout most of the time.
	const streamPreviews = [
		{
			streamer: "TroyKomodo",
			title: "working on https://github.com/ScuffleTV/scuffle",
			tags: ["Programming", "English"],
			viewers: 1,
			avatar:
				"https://static-cdn.jtvnw.net/jtv_user_pictures/3773bfdd-110b-4911-b914-6f04362a1331-profile_image-70x70.png",
			preview: "/troykomodo-preview.png",
		},
		{
			streamer: "BTSSmash",
			title: "RERUN: Sparg0 vs MuteAce - Group B Ultimate Summit 6 - SSBU Singles | Cloud vs Peach",
			tags: ["Just Chatting", "English", "reacts", "memes"],
			viewers: 1100,
			avatar:
				"https://static-cdn.jtvnw.net/jtv_user_pictures/ffdc21f2-f4f9-4ee7-b94d-d7b87df7edc3-profile_image-70x70.jpg",
			preview: "/btssmash-preview.png",
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
			streamer: "PewDiePie",
			title: "Enjoy @PewDiePie ∞  Stream (See link in description of latest video for PROOF)",
			tags: ["English"],
			viewers: 1_200_000,
			avatar:
				"https://cdn.7tv.app/user/641c5678d9a6d799492574c9/av_643efce7400b6139d0908507/2x.webp",
			preview: "/pewdiepie-preview.png",
		},
		{
			streamer: "Bob Ross",
			title: "A Happy Little Weekend Marathon! - The Joy of Painting with Bob Ross",
			tags: ["English", "Painting", "ASMR"],
			viewers: 1_200_000,
			avatar:
				"https://static-cdn.jtvnw.net/jtv_user_pictures/bobross-profile_image-0b9dd167a9bb16b5-70x70.jpeg",
			preview: "/bobross-preview.png",
		},
		{
			streamer: "Ottomated",
			title: "Ludwig told me to re-make Steam from scratch...",
			tags: ["Programming", "English", "React", "NodeJS"],
			viewers: 1,
			avatar:
				"https://static-cdn.jtvnw.net/jtv_user_pictures/d8c28a0b-c232-4c46-88e9-87b14c29eeb0-profile_image-70x70.png",
			preview: "/ottomated-preview.png",
		},
		{
			streamer: "Supinic",
			title: "super secret smol sunday stream",
			tags: ["Just Chatting", "English"],
			viewers: 1100,
			avatar:
				"https://static-cdn.jtvnw.net/jtv_user_pictures/supinic-profile_image-310328b1ff949bf8-70x70.png",
			preview: "/supinic-preview.png",
		},
		{
			streamer: "NymN",
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
			streamer: "rainbolt",
			title: "GEOGUESSER WITH CHAT",
			tags: ["Geoguesser", "English", "interact", "bigbrain"],
			viewers: 100,
			avatar:
				"https://static-cdn.jtvnw.net/jtv_user_pictures/1310788d-3ea3-488a-8d77-5585a97dfe90-profile_image-300x300.png",
			preview: "/rainbolt-preview.png",
		},
		{
			streamer: "AMOURANTH",
			title: "🟢--DROPS ON--🔥 + COSPLAY! 💦 ! s--> my fun links",
			tags: ["Just Chatting", "English", "reacts", "memes"],
			viewers: 1100,
			avatar:
				"https://static-cdn.jtvnw.net/jtv_user_pictures/7f349bf3-ada7-4486-a0f1-a6e055b68fca-profile_image-70x70.png",
			preview: "/amouranth-preview.png",
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
	<meta name="description" content="Scuffle - open-source live-streaming platform" />
	<meta name="keywords" content="scuffle, live, stream, watch" />

	<!-- Open Graph -->
	<meta property="og:type" content="website" />
	<meta property="og:title" content="Scuffle" />
	<meta property="og:description" content="Scuffle - open-source live-streaming platform" />
	<meta property="og:image" content="https://scuffle.tv/favicon.ico" />
	<meta property="og:image:alt" content="Scuffle Logo" />
	<!-- TODO: Change this when the domain changes -->
	<meta property="og:url" content="https://scuffle.tv/" />
	<meta property="og:site_name" content="Scuffle" />
	<!-- TODO: Change this when localizing -->
	<meta property="og:locale" content="en_US" />
</svelte:head>

<div class="content" aria-label="Page content">
	<BigStreamPreview {...streamPreviews[0]} id="727004a1-7446-4304-868d-ae9bf15b3942" />

	<div class="container">
		<h2 class="title">How about this?</h2>
		<section class="hbt" role="feed" aria-label="Stream previews">
			{#each streamPreviews.slice(1) as preview}
				<article>
					<SmallStreamPreview {...preview} />
				</article>
			{/each}
		</section>
		<button class="show-more">
			<h4>show more</h4>
			<Fa icon={faChevronDown} size="1x" />
			<hr />
		</button>
	</div>

	<div class="container">
		<h2 class="title">Categories</h2>
		<section class="categories" role="feed" aria-label="Categories">
			{#each categories as category}
				<article>
					<CategoryCard {...category} />
				</article>
			{/each}
		</section>
	</div>
</div>

<style lang="scss">
	@import "../assets/styles/variables.scss";

	.content {
		grid-row: 2;
		grid-column: 2 / -1;
		overflow-y: auto;

		display: flex;
		flex-direction: column;
		gap: 2rem;
		align-items: center;

		padding: 1rem;

		/* Background Gradient */
		background: radial-gradient(
			75% 50% at 0% 0%,
			rgba($primaryColor, 0.05) 0%,
			rgba($bgColor, 0) 100%
		);

		.container {
			max-width: 125rem;
			width: 100%;
		}
	}

	.show-more {
		width: 100%;
		display: flex;
		align-items: center;
		gap: 0.5rem;
		cursor: pointer;
		color: $primaryColor;

		margin-top: 0.5rem;

		&:hover,
		&:focus-visible {
			color: $primaryColorLight;
		}

		& > h4 {
			margin: 0;
			font-size: 0.95rem;
			font-weight: 400;
		}

		& > hr {
			border-style: solid;
			flex-grow: 1;
			color: $bgColorLight;
		}
	}

	.title {
		font-size: 1.25rem;
		font-weight: 500;
		color: $textColor;
		font-family: $sansFont;

		margin-bottom: 0.5rem;
	}

	.hbt {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(18rem, 1fr));
		gap: 2rem;
		max-width: 125rem;
	}

	.categories {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(9.5rem, 1fr));
		gap: 2rem;
		max-width: 125rem;
	}
</style>
