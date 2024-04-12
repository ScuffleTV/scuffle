<script lang="ts">
	import Fa from "svelte-fa";
	import { faChevronLeft, faChevronRight } from "@fortawesome/free-solid-svg-icons";
	import CategoryCard from "$/components/home/category-card.svelte";
	import SmallStreamPreview from "$/components/home/small-stream-preview.svelte";
	import BigStreamPreview from "$/components/home/big-stream-preview.svelte";
	import Logo from "$/components/icons/logo.svelte";
	import { onMount } from "svelte";
	import { PUBLIC_BASE_URL } from "$env/static/public";
	import type { User } from "$/gql/graphql";
	import ShowMore from "$/components/show-more.svelte";

	// We should always load 13 previews because that means that we show 12 previews in the "How about this?" section
	// 12 is a nice number because it's divisible by 2, 3, 4, and 6 which means that it fills all rows in the grid layout most of the time.
	const streamPreviews = [
		{
			user: {
				id: "01H8WMQ6EPH7YFM1PQTJF81TT7",
				username: "troykomodo",
				displayName: "TroyKomodo",
				channel: {
					title: "working on https://github.com/ScuffleTV/scuffle",
					live: {
						liveViewerCount: 1,
					},
				},
			},
			avatar:
				"https://static-cdn.jtvnw.net/jtv_user_pictures/3773bfdd-110b-4911-b914-6f04362a1331-profile_image-70x70.png",
			preview: "/troykomodo-preview.png",
		},
		{
			user: {
				id: "01H8WMQ6EPH7YFM1PQTJF81TT7",
				username: "btssmash",
				displayName: "BTSSmash",
				channel: {
					title:
						"RERUN: Sparg0 vs MuteAce - Group B Ultimate Summit 6 - SSBU Singles | Cloud vs Peach",
					live: {
						liveViewerCount: 1100,
					},
				},
			},
			avatar:
				"https://static-cdn.jtvnw.net/jtv_user_pictures/ffdc21f2-f4f9-4ee7-b94d-d7b87df7edc3-profile_image-70x70.jpg",
			preview: "/btssmash-preview.png",
		},
		{
			user: {
				id: "01H8WMQ6EPH7YFM1PQTJF81TT7",
				username: "xqc",
				displayName: "xQc",
				channel: {
					title: "LIVE👏CLICK👏NOW👏DRAMA👏NEWS👏PAGMAN",
					live: {
						liveViewerCount: 1100,
					},
				},
			},
			avatar:
				"https://static-cdn.jtvnw.net/jtv_user_pictures/xqc-profile_image-9298dca608632101-300x300.jpeg",
			preview: "/xqc-preview.png",
		},
		{
			user: {
				id: "01H8WMQ6EPH7YFM1PQTJF81TT7",
				username: "pewdiepie",
				displayName: "PewDiePie",
				channel: {
					title: "Enjoy @PewDiePie ∞  Stream (See link in description of latest video for PROOF)",
					live: {
						liveViewerCount: 1100,
					},
				},
			},
			avatar:
				"https://cdn.7tv.app/user/641c5678d9a6d799492574c9/av_643efce7400b6139d0908507/2x.webp",
			preview: "/pewdiepie-preview.png",
		},
		{
			user: {
				id: "01H8WMQ6EPH7YFM1PQTJF81TT7",
				username: "bobross",
				displayName: "Bob Ross",
				channel: {
					title: "A Happy Little Weekend Marathon! - The Joy of Painting with Bob Ross",
					live: {
						liveViewerCount: 1100,
					},
				},
			},
			avatar:
				"https://static-cdn.jtvnw.net/jtv_user_pictures/bobross-profile_image-0b9dd167a9bb16b5-70x70.jpeg",
			preview: "/bobross-preview.png",
		},
		{
			user: {
				id: "01H8WMQ6EPH7YFM1PQTJF81TT7",
				username: "ottomated",
				displayName: "Ottomated",
				channel: {
					title: "Ludwig told me to re-make Steam from scratch...",
					live: {
						liveViewerCount: 1100,
					},
				},
			},
			avatar:
				"https://static-cdn.jtvnw.net/jtv_user_pictures/d8c28a0b-c232-4c46-88e9-87b14c29eeb0-profile_image-70x70.png",
			preview: "/ottomated-preview.png",
		},
		{
			user: {
				id: "01H8WMQ6EPH7YFM1PQTJF81TT7",
				username: "supinic",
				displayName: "Supinic",
				channel: {
					title: "super secret smol sunday stream",
					live: {
						liveViewerCount: 1100,
					},
				},
			},
			avatar:
				"https://static-cdn.jtvnw.net/jtv_user_pictures/supinic-profile_image-310328b1ff949bf8-70x70.png",
			preview: "/supinic-preview.png",
		},
		{
			user: {
				id: "01H8WMQ6EPH7YFM1PQTJF81TT7",
				username: "nymn",
				displayName: "NymN",
				channel: {
					title: "PotFriend Fan Club | !twitter !pobox !youtube",
					live: {
						liveViewerCount: 1100,
					},
				},
			},
			avatar:
				"https://static-cdn.jtvnw.net/jtv_user_pictures/nymn-profile_image-d52821b50793580f-300x300.jpeg",
			preview: "/nymn-preview.png",
		},
		{
			user: {
				id: "01H8WMQ6EPH7YFM1PQTJF81TT7",
				username: "realsport",
				displayName: "realSport",
				channel: {
					title: "REAL SPORTS WITH BRYANT GUMBEL",
					live: {
						liveViewerCount: 1100,
					},
				},
			},
			avatar:
				"https://static-cdn.jtvnw.net/jtv_user_pictures/4a1b51a9-0094-4cb8-bdf4-dba7c7e64dea-profile_image-300x300.png",
			preview: "/realsport-preview.png",
		},
		{
			user: {
				id: "01H8WMQ6EPH7YFM1PQTJF81TT7",
				username: "rainbolt",
				displayName: "rainbolt",
				channel: {
					title: "GEOGUESSER WITH CHAT",
					live: {
						liveViewerCount: 1100,
					},
				},
			},
			avatar:
				"https://static-cdn.jtvnw.net/jtv_user_pictures/1310788d-3ea3-488a-8d77-5585a97dfe90-profile_image-300x300.png",
			preview: "/rainbolt-preview.png",
		},
		{
			user: {
				id: "01H8WMQ6EPH7YFM1PQTJF81TT7",
				username: "amouranth",
				displayName: "AMOURANTH",
				channel: {
					title: "🟢--DROPS ON--🔥 + COSPLAY! 💦 ! s--> my fun links",
					live: {
						liveViewerCount: 1100,
					},
				},
			},
			avatar:
				"https://static-cdn.jtvnw.net/jtv_user_pictures/7f349bf3-ada7-4486-a0f1-a6e055b68fca-profile_image-70x70.png",
			preview: "/amouranth-preview.png",
		},
	] as {
		user: User;
		avatar: string;
		preview: string;
	}[];

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

	function calculatePlaceholderSpan(
		hbtGridWidth?: number,
		previewWidth?: number,
	): number | undefined {
		if (!hbtGridWidth || !previewWidth) {
			return undefined;
		}
		const gapSize = 2 * 16;
		const numColumns = Math.round((hbtGridWidth + gapSize) / (previewWidth + gapSize));
		const numThumbnails = streamPreviews.length - 1;
		const remainingColumns = numColumns - (numThumbnails % numColumns);
		if (remainingColumns === numColumns || remainingColumns === 0) {
			return undefined;
		}
		return remainingColumns;
	}

	let hbtGridWidth: number;
	let previewWidth: number;
	$: placeholderSpan = calculatePlaceholderSpan(hbtGridWidth, previewWidth);

	let categoriesSlider: HTMLElement;
	let scrollLeft: number = 0;
	let scrollWidth: number = 0;

	const categoryWidth = (9.5 + 2) * 16;
	function slideCategories(direction: number) {
		if (categoriesSlider) {
			// Calculate how many categories fit on the screen
			// Don't scroll more than 3
			const num = Math.min(Math.floor(window.innerWidth / categoryWidth), 3);
			categoriesSlider.scrollBy({ left: direction * num * categoryWidth, behavior: "smooth" });
		}
	}

	function onScroll() {
		if (categoriesSlider) {
			scrollLeft = categoriesSlider.scrollLeft;
			scrollWidth = categoriesSlider.scrollWidth;
		}
	}

	onMount(onScroll);
</script>

<svelte:head>
	<title>Scuffle - Home</title>

	<!-- Open Graph -->
	<meta property="og:title" content="Scuffle" />
	<meta property="og:description" content="Scuffle - open-source live-streaming platform" />
	<meta property="og:image" content="{PUBLIC_BASE_URL}/banner.jpeg" />
	<meta property="og:image:alt" content="Scuffle Banner" />
</svelte:head>

<div class="content" aria-label="Page content">
	<div class="bg-gradient"></div>

	<BigStreamPreview
		user={streamPreviews[0].user}
		avatar={streamPreviews[0].avatar}
		preview={streamPreviews[0].preview}
		tags={["sometag"]}
	/>

	<div class="container hbt-container">
		<h2 class="title" id="hbt-title">How about this?</h2>
		<section class="hbt" role="feed" aria-labelledby="hbt-title" bind:offsetWidth={hbtGridWidth}>
			{#each streamPreviews.slice(1) as preview, i}
				{#if i === 0}
					<article bind:offsetWidth={previewWidth}>
						<SmallStreamPreview
							user={preview.user}
							avatar={preview.avatar}
							preview={preview.preview}
						/>
					</article>
				{:else}
					<article>
						<SmallStreamPreview
							user={preview.user}
							avatar={preview.avatar}
							preview={preview.preview}
						/>
					</article>
				{/if}
			{/each}
			<article
				class="preview-placeholder"
				class:hidden={!placeholderSpan}
				style="grid-column: auto / span {placeholderSpan};"
			>
				<Logo height={44} />
				<span>Go live to appear here</span>
			</article>
		</section>
		<ShowMore />
	</div>

	<div class="container categories-container">
		<h2 class="title" id="categories-title">Categories</h2>
		<section
			class="categories"
			role="feed"
			aria-labelledby="categories-title"
			bind:this={categoriesSlider}
			on:scroll={onScroll}
		>
			{#each categories as category}
				<article>
					<CategoryCard {...category} />
				</article>
			{/each}
		</section>
		{#if scrollLeft > 16 * 9.5}
			<button on:click={() => slideCategories(-1)} class="slide-button left">
				<Fa icon={faChevronLeft} />
			</button>
		{/if}
		{#if categoriesSlider && scrollWidth - (scrollLeft + categoriesSlider.offsetWidth) > 16 * 9.5}
			<button on:click={() => slideCategories(1)} class="slide-button right">
				<Fa icon={faChevronRight} />
			</button>
		{/if}
	</div>
</div>

<style lang="scss">
	@import "../../assets/styles/variables.scss";

	@keyframes scale-up {
		from {
			transform: scale(0);
		}
		to {
			transform: scale(1);
		}
	}

	.content {
		grid-area: content;
		overflow-y: auto;

		display: flex;
		flex-direction: column;
		gap: 2rem;
		align-items: center;

		position: relative;

		.bg-gradient {
			pointer-events: none;

			position: absolute;
			top: 0;
			left: 0;
			right: 0;
			bottom: 0;

			background: radial-gradient(
				75% 50% at 0% 0%,
				rgba($primaryColor, 0.05) 0%,
				rgba($bgColor, 0) 100%
			);

			transform-origin: top left;
			animation: scale-up 0.5s cubic-bezier(0.19, 1, 0.22, 1) forwards;
		}

		.container {
			/*
			    A category card is 9.5rem in width. With a max-width of 126.5rem exactly 11 cards fit next to each other with a gap of 2rem in between and a padding of 1rem left and right.
			    11 * 9.5 + 10 * 2 + 1 + 1 = 126.5
			*/
			max-width: 126.5rem;
			width: 100%;

			&.hbt-container {
				padding: 0 1rem;
			}
		}
	}

	.title {
		font-size: 1.5rem;
		font-weight: 500;
		color: $textColor;
		font-family: $sansFont;

		margin-bottom: 1rem;
	}

	.hbt {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(18rem, 1fr));
		gap: 2rem;

		& > .preview-placeholder {
			display: flex;
			flex-direction: column;
			justify-content: center;
			align-items: center;
			gap: 0.5rem;

			border-radius: 0.5rem;

			background: linear-gradient(
				118deg,
				rgba($primaryColor, 0.05) 0%,
				rgba(255, 255, 255, 0.05) 100%
			);

			font-weight: 500;

			&.hidden {
				display: none;
			}
		}
	}

	.categories-container {
		position: relative;

		& > .title {
			margin-left: 1rem;
			/* -2rem because categories has padding-top of 2rem */
			margin-bottom: -1rem;
		}

		& > .categories {
			display: flex;
			gap: 2rem;
			overflow-x: auto;

			/* Hide scrollbar */
			-ms-overflow-style: none; /* IE and Edge */
			scrollbar-width: none; /* Firefox */
			&::-webkit-scrollbar {
				display: none; /* Chrome, Safari, Opera */
			}

			padding: 1rem;
			padding-top: 2rem;

			& > article {
				width: 9.5rem;
			}
		}

		& > .slide-button {
			position: absolute;

			top: 0;
			bottom: 0;

			width: 5rem;

			color: $textColor;
			font-size: 2rem;

			&.left {
				left: 0;
				background: linear-gradient(270deg, rgba($bgColor, 0) 0%, rgba($bgColor, 1) 100%);
			}

			&.right {
				right: 0;
				background: linear-gradient(90deg, rgba($bgColor, 0) 0%, rgba($bgColor, 1) 100%);
			}
		}
	}

	@media screen and (max-width: $mobileBreakpoint) {
		.hbt {
			gap: 1rem;
		}
	}
</style>
