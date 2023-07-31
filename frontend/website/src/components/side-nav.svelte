<script lang="ts">
	import Home from "$icons/home.svelte";
	import Following from "$icons/following.svelte";

	import { sideNavOpen } from "$store/layout";
	import SideNavStreamerCard from "$components/side-nav/streamer-card.svelte";

	const followedChannels = [
		{
			name: "Place",
			avatar:
				"https://static-cdn.jtvnw.net/jtv_user_pictures/ad9f645d-2832-4c02-abda-c513a1fc4906-profile_image-70x70.png",
			game: "Art",
			viewers: 232,
		},
		{
			name: "xQc",
			avatar:
				"https://static-cdn.jtvnw.net/jtv_user_pictures/xqc-profile_image-9298dca608632101-300x300.jpeg",
			game: "CS:GO",
			viewers: 1300,
		},
		{
			name: "pokelawls",
			avatar:
				"https://static-cdn.jtvnw.net/jtv_user_pictures/aa68742d-ae1f-4fb7-9d0c-e1756d5204b0-profile_image-300x300.jpg",
			game: "Counter-Strike: Global Offensive",
			viewers: 103_000,
		},
		{
			name: "fruitBerries",
			avatar:
				"https://static-cdn.jtvnw.net/jtv_user_pictures/4a1b51a9-0094-4cb8-bdf4-dba7c7e64dea-profile_image-300x300.png",
			game: "CS:GO 2",
			viewers: null,
		},
	];

	const recommendedChannels = [
		{
			name: "PewDiePie",
			avatar:
				"https://static-cdn.jtvnw.net/jtv_user_pictures/5940a887-fe7a-48b4-bea5-50b7a1a8e7e3-profile_image-300x300.png",
			game: "Just Hottubbing",
			viewers: 232,
		},
		{
			name: "BobRoss",
			avatar:
				"https://static-cdn.jtvnw.net/jtv_user_pictures/bobross-profile_image-0b9dd167a9bb16b5-300x300.jpeg",
			game: "Art",
			viewers: 1300,
		},
		{
			name: "Ottomated",
			avatar:
				"https://static-cdn.jtvnw.net/jtv_user_pictures/d8c28a0b-c232-4c46-88e9-87b14c29eeb0-profile_image-300x300.png",
			game: "Dev",
			viewers: 103_000,
		},
		{
			name: "forsen",
			avatar:
				"https://static-cdn.jtvnw.net/jtv_user_pictures/forsen-profile_image-48b43e1e4f54b5c8-300x300.png",
			game: "Minecraft",
			viewers: 1300,
		},
		{
			name: "LIRIK",
			avatar:
				"https://static-cdn.jtvnw.net/jtv_user_pictures/38e925fc-0b07-4e1e-82e2-6639e01344f3-profile_image-300x300.png",
			game: "Street Fighter 6",
			viewers: 1300,
		},
	];
</script>

<nav class:collapsed={!$sideNavOpen} class="main-grid">
	<a href="/" class="item selected">
		<Home /><span>home</span>
	</a>
	<a href="/" class="item">
		<Following /><span>following</span>
	</a>

	<hr />

	<h3 class="category">followed channels</h3>

	<div class="streamer-cards followed-channels">
		{#each followedChannels as streamer}
			<SideNavStreamerCard {...streamer} />
		{/each}
	</div>

	<h4 class="show-more">show more</h4>

	<h3 class="category">recommended</h3>

	<div class="streamer-cards recommended-channels">
		{#each recommendedChannels as streamer}
			<SideNavStreamerCard {...streamer} />
		{/each}
	</div>

	<h4 class="show-more">show more</h4>
</nav>

<style lang="scss">
	@import "../assets/styles/variables.scss";

	.main-grid {
		grid-area: side-nav;
		transition: width 0.2s;
	}

	.streamer-cards {
		display: flex;
		flex-direction: column;
	}

	.item {
		color: #bcbcbc;
		display: grid;
		grid-template-columns: auto 1fr;
		align-items: center;
		text-decoration: none;
		height: 2.5rem;
		column-gap: 0.5rem;
		border: 1px solid transparent;
		padding: 0.5rem 0.75rem;
		margin-left: 0.25rem;
		transition:
			filter 0.2s ease-in-out,
			color 0.2s ease-in-out,
			background-color 0.2s ease-in-out,
			border-color 0.2s ease-in-out;
		border-radius: 1rem;

		> span {
			font-weight: 500;
			line-height: normal;
		}

		&.selected {
			color: white;
			filter: drop-shadow(0 0 5px rgba(255, 255, 255, 0.5));
			&:hover {
				filter: drop-shadow(0 0 5px rgba(255, 255, 255, 1));
			}
		}

		&:hover:not(.selected) {
			color: white;
			background-color: #2b292e27;
			border-color: #bcbcbc27;
		}
	}

	nav {
		position: sticky;
		top: $topNavHeight;
		height: calc(100vh - #{$topNavHeight});
		z-index: 2;
		width: $sideNavWidth;
		background-color: $bgColor2;
		display: flex;
		flex-direction: column;
	}

	.collapsed {
		.item {
			> span {
				display: none;
			}
			column-gap: 0;
			margin-left: 0;
			padding: 0.5rem;
		}

		.category,
		.show-more,
		.streamer-cards {
			display: none;
		}

		width: 2.75em;
	}

	hr {
		width: 100%;
		border: 0;
		border-bottom: 0.1em solid #252525;
	}

	.category {
		margin: 0;
		padding: 0.5rem 0.75rem;
		font-size: 0.85rem;
		font-weight: 500;
		color: #585858;
		text-transform: uppercase;
	}

	.show-more {
		margin: 0;
		padding: 0.5rem 0.75rem;
		font-size: 0.95rem;
		font-weight: 400;
		color: #ff5a39;
		cursor: pointer;
		&:hover {
			color: #f47e66;
		}
	}
</style>
