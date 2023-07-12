<script lang="ts">
	import Fa from "svelte-fa";
	import { faHouse, faPersonWalking } from "@fortawesome/free-solid-svg-icons";

	import { sideNavCollapsed, sideNavHidden } from "$store/layout";
	import SideNavStreamerCard from "$components/side-nav/streamer-card.svelte";
	import { page } from "$app/stores";

	const followedChannels = [
		{
			username: "place",
			displayName: "Place",
			avatar:
				"https://static-cdn.jtvnw.net/jtv_user_pictures/ad9f645d-2832-4c02-abda-c513a1fc4906-profile_image-70x70.png",
			game: "Art",
			viewers: 232,
		},
		{
			username: "xqc",
			displayName: "xQc",
			avatar:
				"https://static-cdn.jtvnw.net/jtv_user_pictures/xqc-profile_image-9298dca608632101-300x300.jpeg",
			game: "CS:GO",
			viewers: 1300,
		},
		{
			username: "pokelawls",
			displayName: "pokelawls",
			avatar:
				"https://static-cdn.jtvnw.net/jtv_user_pictures/aa68742d-ae1f-4fb7-9d0c-e1756d5204b0-profile_image-300x300.jpg",
			game: "Counter-Strike: Global Offensive",
			viewers: 103_000,
		},
		{
			username: "fruitBerries",
			displayName: "fruitBerries",
			avatar:
				"https://static-cdn.jtvnw.net/jtv_user_pictures/4a1b51a9-0094-4cb8-bdf4-dba7c7e64dea-profile_image-300x300.png",
			game: "CS:GO 2",
			viewers: null,
		},
	];

	const recommendedChannels = [
		{
			username: "pewdiepie",
			displayName: "PewDiePie",
			avatar: "https://static-cdn.jtvnw.net/emoticons/v2/115234/default/dark/1.0",
			game: "Just Hottubbing",
			viewers: 232,
		},
		{
			username: "bobross",
			displayName: "BobRoss",
			avatar:
				"https://static-cdn.jtvnw.net/jtv_user_pictures/bobross-profile_image-0b9dd167a9bb16b5-300x300.jpeg",
			game: "Art",
			viewers: 1300,
		},
		{
			username: "ottomated",
			displayName: "Ottomated",
			avatar:
				"https://static-cdn.jtvnw.net/jtv_user_pictures/d8c28a0b-c232-4c46-88e9-87b14c29eeb0-profile_image-300x300.png",
			game: "Dev",
			viewers: 103_000,
		},
		{
			username: "forsen",
			displayName: "forsen",
			avatar:
				"https://static-cdn.jtvnw.net/jtv_user_pictures/forsen-profile_image-48b43e1e4f54b5c8-300x300.png",
			game: "Minecraft",
			viewers: 1300,
		},
		{
			username: "lirik",
			displayName: "LIRIK",
			avatar:
				"https://static-cdn.jtvnw.net/jtv_user_pictures/38e925fc-0b07-4e1e-82e2-6639e01344f3-profile_image-300x300.png",
			game: "Street Fighter 6",
			viewers: 1300,
		},
	];
</script>

<nav
	class:collapsed={$sideNavCollapsed}
	class:hidden={$sideNavHidden}
	id="side-nav"
	aria-label="Side bar"
>
	<div>
		<a href="/" class="item" class:selected={$page.url.pathname === "/"}>
			<Fa icon={faHouse} fw size="1.2x" /><span>home</span>
		</a>
		<a href="/following" class="item" class:selected={$page.url.pathname === "/following"}>
			<Fa icon={faPersonWalking} fw size="1.2x" /><span>following</span>
		</a>
		<hr />
	</div>

	<div>
		<h3 class="category">following</h3>
		<div class="streamer-cards followed-channels">
			{#each followedChannels as streamer}
				<SideNavStreamerCard {...streamer} />
			{/each}
		</div>
		<button class="show-more">show more</button>
	</div>

	<div>
		<h3 class="category">recommended</h3>
		<div class="streamer-cards recommended-channels">
			{#each recommendedChannels as streamer}
				<SideNavStreamerCard {...streamer} />
			{/each}
		</div>
		<button class="show-more">show more</button>
	</div>
</nav>

<style lang="scss">
	@import "../assets/styles/variables.scss";

	nav {
		grid-area: side-nav;

		max-width: $sideNavWidth;
		background-color: $bgColor2;
		border-right: 0.125rem solid $bgColorLight;

		display: flex;
		flex-direction: column;
		gap: 1rem;

		&.hidden {
			display: none;
		}
	}

	.streamer-cards {
		display: flex;
		flex-direction: column;
	}

	.item {
		display: flex;
		align-items: center;
		gap: 0.5rem;

		color: $textColorLight;
		text-decoration: none;
		padding: 0.75rem;
		padding-left: 0.625rem;
		font-weight: 500;
		border-left: 0.125rem solid transparent;

		&.selected {
			border-color: $primaryColor;
			color: $textColor;
		}

		&:hover,
		&:focus-visible {
			background-color: $bgColorLight;
		}
	}

	.collapsed {
		.item {
			& > span {
				display: none;
			}
		}

		.category,
		.show-more,
		.streamer-cards {
			display: none;
		}
	}

	hr {
		width: 100%;
		border-style: solid;
		color: $bgColorLight;
		margin: 0;
	}

	.category {
		margin: 0;
		padding: 0.5rem 0.75rem;
		font-size: 0.85rem;
		font-weight: 500;
		color: $textColorLight;
		text-transform: uppercase;
	}

	.show-more {
		margin: 0;
		padding: 0.5rem 0.75rem;
		font-size: 0.95rem;
		font-weight: 400;
		color: $primaryColor;
		cursor: pointer;
		&:hover,
		&:focus-visible {
			color: $primaryColorLight;
		}
	}
</style>
