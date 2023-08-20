<script lang="ts">
	import Chatroom from "$/components/chat/chatroom.svelte";
	import type { PageData } from "./$types";
	import Tag from "$/components/tag.svelte";
	import Player from "$/components/player.svelte";
	import Fa from "svelte-fa";
	import { faPersonWalking, faPersonRunning } from "@fortawesome/free-solid-svg-icons";
	import { viewersToString } from "$/lib/utils";

	export let data: PageData;

	const { user, stream } = data;
</script>

<svelte:head>
	<title>Scuffle - {user.displayName}</title>
	<meta name="description" content="Watch {user.displayName} live on Scuffle" />
	<meta
		name="keywords"
		content="scuffle, live, stream, watch, {user.displayName}, {user.username}"
	/>

	<!-- Open Graph -->
	<meta property="og:type" content="website" />
	<meta property="og:title" content="Scuffle - {user.displayName}" />
	<meta property="og:description" content="Watch {user.displayName} live on Scuffle" />
	<meta
		property="og:image"
		content="https://static-cdn.jtvnw.net/jtv_user_pictures/3773bfdd-110b-4911-b914-6f04362a1331-profile_image-70x70.png"
	/>
	<meta property="og:image:alt" content="{user.displayName}'s profile picture" />
	<!-- TODO: Change this when the domain changes -->
	<meta property="og:url" content="https://scuffle.tv/{user.displayName}" />
	<meta property="og:site_name" content="Scuffle" />
	<!-- TODO: Change this when localizing -->
	<meta property="og:locale" content="en_US" />
</svelte:head>

<div class="content">
	{#if stream}
		<div class="player-container">
			<Player streamId={stream.id} />
			<div class="under-player">
				<div class="row title-row">
					<h1 class="title">working on https://github.com/scuffletv</h1>
					<div class="stream-info">
						<span class="viewers">{viewersToString(103000)}</span>
						<span class="time">01:20:11</span>
					</div>
				</div>
				<div class="row">
					<div>
						<div class="user">
							<img
								class="avatar"
								src="https://static-cdn.jtvnw.net/jtv_user_pictures/3773bfdd-110b-4911-b914-6f04362a1331-profile_image-70x70.png"
								alt="Avatar"
							/>
							<h1 class="name">{user.displayName}</h1>
							<span class="game">Software and Game Development</span>
						</div>
						<button class="button primary">
							<Fa icon={faPersonWalking} size="1.2x" />
							<span>Follow</span>
						</button>
					</div>
					<button class="button primary">
						<Fa icon={faPersonRunning} size="1.2x" />
						<span>Subscribe</span>
					</button>
				</div>
				<div class="tags">
					<Tag content="English" />
					<Tag content="open source" />
					<Tag content="streaming" />
				</div>
			</div>
		</div>
	{:else}
		<span>Offline</span>
	{/if}
</div>

<Chatroom channelId={user.id} />

<style lang="scss">
	@import "../../assets/styles/variables.scss";

	.content {
		grid-area: content;
		overflow-y: scroll;

		display: flex;

		/* Hide scrollbar */
		&::-webkit-scrollbar {
			display: none;
		}
		-ms-overflow-style: none;
		scrollbar-width: none;

		& > .player-container {
			flex-grow: 1;
		}
	}

	.under-player {
		padding: 0.25rem 0.5rem;
		font-family: $sansFont;
		font-weight: 500;

		display: flex;
		gap: 0.75rem;
		flex-direction: column;

		.button {
			padding: 0.25rem 0.5rem;
			font-weight: 500;

			display: flex;
			align-items: center;
			gap: 0.4rem;
		}

		.row {
			display: flex;
			align-items: center;
			justify-content: space-between;
			gap: 0.5rem;

			& > div {
				display: flex;
				align-items: center;
				gap: 0.5rem;
			}
		}

		.title-row {
			flex-wrap: wrap;
		}

		.title {
			font-size: 1.25rem;
		}

		.user {
			display: grid;
			grid-template-areas: "avatar name" "avatar game";
			column-gap: 0.5rem;
			align-items: center;

			& > .avatar {
				grid-area: avatar;
				width: 2.5rem;
				height: 2.5rem;
				border-radius: 50%;
			}

			& > .name {
				grid-area: name;
				font-size: 1.25rem;
				overflow: hidden;
				text-overflow: ellipsis;
				white-space: nowrap;
			}

			& > .game {
				grid-area: game;
				font-size: 0.875rem;
				color: $textColorLight;
				overflow: hidden;
				text-overflow: ellipsis;
				white-space: nowrap;
			}
		}

		.tags {
			display: flex;
			gap: 0.5rem;
			flex-wrap: wrap;
		}

		.stream-info {
			flex-wrap: wrap;

			.viewers,
			.time {
				white-space: nowrap;

				&::before {
					content: "";
					display: inline-block;
					width: 0.4rem;
					height: 0.4rem;
					margin-right: 0.4rem;
					margin-bottom: 0.1rem;
				}
			}

			.viewers::before {
				background-color: $liveColor;
				border-radius: 50%;
			}

			.time::before {
				background-color: $textColor;
			}
		}
	}
</style>
