<script lang="ts">
	import Fa from "svelte-fa";
	import { faCircleQuestion } from "@fortawesome/free-regular-svg-icons";
	import { faHouse, faUser } from "@fortawesome/free-solid-svg-icons";

	import { sideNavCollapsed, sideNavHidden } from "$store/layout";
	import SideNavStreamerCard from "$components/side-nav/streamer-card.svelte";
	import { page } from "$app/stores";
	import { getContextClient } from "@urql/svelte";
	import { graphql } from "$/gql";
	import { userId } from "$/store/auth";
	import LoadingStreamerCard from "./side-nav/loading-streamer-card.svelte";

	const client = getContextClient();

	async function queryFollowing(userId: string | null, limit: number) {
		if (!userId) return null;

		const res = await client
			.query(
				graphql(`
					query UserFollowing($userId: ULID!, $limit: Int!) {
						user {
							following: following(id: $userId, limit: $limit) {
								id
								username
								displayName
								displayColor {
									color
									hue
									isGray
								}
								profilePicture {
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
								channel {
									live {
										liveViewerCount
									}
									category {
										name
									}
								}
							}
						}
					}
				`),
				{ userId, limit },
				{ requestPolicy: "network-only" },
			)
			.toPromise();

		return res.data?.user.following;
	}

	let followingLimit = 5;
	let following: Awaited<ReturnType<typeof queryFollowing>>;

	$: queryFollowing($userId, followingLimit).then((res) => {
		following = res;
	});

	function showMoreFollowing() {
		followingLimit += 5;
	}

	// const recommendedChannels = [
	// 	{
	// 		username: "pewdiepie",
	// 		displayName: "PewDiePie",
	// 		avatar: "https://static-cdn.jtvnw.net/emoticons/v2/115234/default/dark/1.0",
	// 		game: "Just Hottubbing",
	// 		viewers: 232,
	// 	},
	// 	{
	// 		username: "bobross",
	// 		displayName: "BobRoss",
	// 		avatar:
	// 			"https://static-cdn.jtvnw.net/jtv_user_pictures/bobross-profile_image-0b9dd167a9bb16b5-300x300.jpeg",
	// 		game: "Art",
	// 		viewers: 1300,
	// 	},
	// 	{
	// 		username: "ottomated",
	// 		displayName: "Ottomated",
	// 		avatar:
	// 			"https://static-cdn.jtvnw.net/jtv_user_pictures/d8c28a0b-c232-4c46-88e9-87b14c29eeb0-profile_image-300x300.png",
	// 		game: "Dev",
	// 		viewers: 103_000,
	// 	},
	// 	{
	// 		username: "forsen",
	// 		displayName: "forsen",
	// 		avatar:
	// 			"https://static-cdn.jtvnw.net/jtv_user_pictures/forsen-profile_image-48b43e1e4f54b5c8-300x300.png",
	// 		game: "Minecraft",
	// 		viewers: 1300,
	// 	},
	// 	{
	// 		username: "lirik",
	// 		displayName: "LIRIK",
	// 		avatar:
	// 			"https://static-cdn.jtvnw.net/jtv_user_pictures/38e925fc-0b07-4e1e-82e2-6639e01344f3-profile_image-300x300.png",
	// 		game: "Street Fighter 6",
	// 		viewers: 1300,
	// 	},
	// ];
</script>

<nav
	class:collapsed={$sideNavCollapsed}
	class:hidden={$sideNavHidden}
	id="side-nav"
	aria-label="Side bar"
>
	<div class="link-container">
		<a href="/" class="item" class:selected={$page.url.pathname === "/"}>
			<Fa icon={faHouse} fw size="1.2x" /><span class="hide-on-mobile">home</span>
		</a>
		<a href="/following" class="item" class:selected={$page.url.pathname === "/following"}>
			<Fa icon={faUser} fw size="1.2x" /><span class="hide-on-mobile">following</span>
		</a>
	</div>

	{#if following}
		<div class="container hide-on-mobile">
			<h3 class="heading">following</h3>
			<div class="streamer-cards">
				{#each following as user}
					<SideNavStreamerCard {...user} collapsed={$sideNavCollapsed} />
				{/each}
			</div>
			{#if followingLimit <= following.length}
				<button class="show-more" on:click={showMoreFollowing}>show more</button>
			{/if}
		</div>
	{:else if following === undefined}
		<div class="container hide-on-mobile">
			<div class="loading heading"></div>
			<div class="streamer-cards">
				{#each Array(followingLimit) as _}
					<LoadingStreamerCard collapsed={$sideNavCollapsed} />
				{/each}
			</div>
		</div>
	{/if}

	<!-- <div>
		<h3 class="heading">recommended</h3>
		<div class="streamer-cards">
			{#each recommendedChannels as streamer}
				<SideNavStreamerCard {...streamer} />
			{/each}
		</div>
		<button class="show-more">show more</button>
	</div> -->

	<a href="/about" class="about hide-on-mobile">
		<Fa icon={faCircleQuestion} />
		What is Scuffle?
	</a>
</nav>

<style lang="scss">
	@import "../assets/styles/variables.scss";

	nav {
		grid-area: side-nav;

		background-color: $bgColor2;
		border-right: 0.125rem solid $bgColorLight;

		display: flex;
		flex-direction: column;
		gap: 0.5rem;

		overflow-y: auto;

		&.hidden {
			display: none;
		}

		&:not(.collapsed) {
			width: $sideNavWidth;
		}
	}

	.link-container {
		display: flex;
		flex-direction: column;

		width: 100%;

		border-bottom: 1px solid $bgColorLight;

		& > .item {
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
	}

	.collapsed {
		.item {
			justify-content: center;

			& > span {
				display: none;
			}
		}

		.heading,
		.show-more,
		.about {
			display: none;
		}
	}

	.container {
		.streamer-cards {
			display: flex;
			flex-direction: column;
		}

		.heading {
			margin: 0.5rem 0.75rem;
			margin-top: 1rem;
			font-size: 0.85rem;
			font-weight: 500;
			color: $textColorLight;
			text-transform: uppercase;

			&.loading {
				width: 8rem;
				height: 1rem;
				border-radius: 0.5rem;
				background-color: $bgColorLight;
			}
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
	}

	.about {
		margin-top: auto;
		color: $textColorLight;
		text-decoration: none;
		font-size: 0.9rem;
		border-top: 1px solid $borderColor;
		padding: 0.5rem 0.75rem;

		&:hover,
		&:focus-visible {
			color: $textColor;
		}
	}

	@media screen and (max-width: $mobileBreakpoint) {
		nav {
			border: none;
			align-items: center;

			&:not(.collapsed) {
				width: 100%;
			}
		}

		.link-container {
			border-bottom: none;
			flex-direction: row;

			& > .item {
				flex-grow: 1;
				justify-content: center;

				padding: 1rem;
				border-left: none;
				border-bottom: 0.125rem solid transparent;
			}
		}
	}
</style>
