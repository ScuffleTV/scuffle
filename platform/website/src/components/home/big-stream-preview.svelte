<script lang="ts">
	import { viewersToString } from "$/lib/utils";
	import Play from "../icons/player/play.svelte";
	import Tag from "../tag.svelte";
	import Messages, { ChatStatus } from "../chat/messages.svelte";
	import { writable } from "svelte/store";
	import Player from "../player.svelte";
	import type { User } from "$/gql/graphql";

	export let user: User;
	export let avatar: string;
	export let preview: string;
	export let tags: string[];

	let chatStatus = writable(ChatStatus.Connecting);
	let playing = false;

	$: viewers = viewersToString(user.channel.live?.liveViewerCount ?? 0, true);

	function onPlayClick() {
		playing = true;
	}
</script>

<div class="preview" aria-label="{user.displayName} streaming {user.channel.title} with {viewers}">
	<div class="stream-info">
		<a class="user" href="/{user.username}">
			<img src={avatar} alt="User Avatar" />
			<h2>{user.displayName}</h2>
		</a>
		<a class="title" href="/{user.username}">{user.channel.title}</a>
		<div class="tags">
			{#each tags as tag}
				<Tag content={tag} />
			{/each}
		</div>
		<span class="viewers">{viewers}</span>
	</div>
	<a class="video" href="/{user.username}">
		{#if playing && user.channel.live}
			<Player live={user.channel.live} showPip={false} showTheater={false} />
		{:else}
			<img src={preview} alt="Stream Thumbnail" class="blurred" aria-hidden="true" />
			<img src={preview} alt="Stream Thumbnail" class="thumbnail" />
			<button class="play" on:click|preventDefault={onPlayClick}>
				<span class="sr-only">Play</span>
				<Play size={96} />
			</button>
		{/if}
	</a>
	<div class="messages">
		<Messages channelId={user.id} {chatStatus} onlyUserMessages />
	</div>
</div>

<style lang="scss">
	@import "../../assets/styles/variables.scss";

	.preview {
		display: flex;
		justify-content: center;
		align-items: stretch;
		flex-wrap: wrap;
		column-gap: 5rem;
		row-gap: 2rem;

		padding: 2rem;
	}

	.stream-info {
		max-width: 20rem;

		display: flex;
		flex-direction: column;
		justify-content: center;
		gap: 1rem;

		& > .user {
			display: flex;
			align-items: center;
			gap: 0.5rem;
			flex-wrap: wrap;

			color: $textColor;
			text-decoration: none;

			&:hover,
			&:focus-visible {
				& > img {
					transform: scale(1.1);
				}

				& > h2::after {
					transform: scale(1);
				}
			}

			& > img {
				width: 2.5rem;
				height: 2.5rem;
				border-radius: 50%;
				transition: transform 0.25s;
			}

			& > h2 {
				font-size: 2.25rem;
				font-weight: 500;
				position: relative;
				white-space: nowrap;

				&::after {
					content: "";
					display: block;
					position: absolute;
					width: 100%;
					height: 1px;
					background: $textColor;
					transition: transform 0.25s;
					transform: scale(0);
				}
			}
		}

		& > .title {
			color: $textColor;
			text-decoration: none;
			font-weight: 500;
			line-height: 1.4em;
		}

		& > .tags {
			display: flex;
			column-gap: 0.5rem;
			row-gap: 0.25rem;
			flex-wrap: wrap;
		}

		& > .viewers {
			font-size: 0.875rem;
			font-weight: 500;
			color: $textColorLight;

			&::before {
				content: "";
				display: inline-block;
				width: 0.4rem;
				height: 0.4rem;
				background-color: $liveColor;
				border-radius: 50%;
				margin-right: 0.4rem;
				margin-bottom: 0.1rem;
			}
		}
	}

	.video {
		position: relative;
		max-width: 28rem;
		width: 85vw;
		aspect-ratio: 16 / 9;

		& > .thumbnail {
			height: 100%;
			width: 100%;
		}

		& > .blurred {
			height: 100%;
			width: 100%;
			position: absolute;
			top: 0;
			left: 0;

			filter: saturate(1.5) blur(30px);
			z-index: -1;
			transition: filter 0.25s;
		}

		&:hover,
		&:focus-visible {
			& > .blurred {
				filter: saturate(2) blur(50px);
			}
		}

		& > .play {
			position: absolute;
			top: 50%;
			left: 50%;
			transform: translate(-50%, -50%);

			color: $textColor;
			background: none;
			border: none;
		}
	}

	.messages {
		overflow: hidden;
		width: 20rem;
		height: 18rem;

		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		justify-content: flex-end;

		-webkit-mask-image: linear-gradient(to top, black 0%, transparent 90%);
		mask-image: linear-gradient(to top, black 0%, transparent 90%);

		transition: width 0.25s;
	}

	.messages:empty {
		display: none;
	}
</style>
