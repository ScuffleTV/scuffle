<script lang="ts">
	import { faStar } from "@fortawesome/free-solid-svg-icons";
	import Fa from "svelte-fa";
	import Chatroom from "$components/chatroom.svelte";
	import type { PageData } from "./$types";

	export let data: PageData;

	const { user } = data;
</script>

<div class="channel">
	<div class="video-player-container">
		<div class="aspect-ratio" />
		<iframe
			src="https://www.youtube.com/embed/dQw4w9WgXcQ?autoplay=1"
			class="video-player"
			frameborder="0"
			allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture"
			allowfullscreen
			title="Video Player Example"
		/>
	</div>
	<div class="under-player">
		<span class="channel-name">{user.username}</span>
		<span class="category">Counter Strike: Global Offensive</span>
		<div class="right-side">
			<span class="viewer-count">
				25,325
				<svg width="1em" height="1em" fill="currentColor">
					<circle cx="50%" cy="50%" r="50%" />
				</svg>
			</span>
			<button class="follow-button">
				<Fa icon={faStar} />
			</button>
			<button class="subscribe-button"> Subscribe </button>
		</div>
	</div>
	<div class="super-under">some content thats super under the video player</div>
</div>

<Chatroom channelId={user.id} />

<svelte:head>
	<title>Scuffle - {user.username}</title>
</svelte:head>

<style lang="scss">
	@import "../../assets/styles/variables.scss";

	// Setting max-height to 0 and min-height to 100%
	// forces the element to take up the full height of the parent
	// and then if the content is taller than the parent,
	// the overflow-y property will allow the user to scroll
	.channel {
		grid-row: 2 / 2;
		grid-column: 2 / 2;
		min-height: 100%;
		overflow-y: overlay;
		position: relative;
		max-height: 0;
	}

	// This is a hack to make sure that the video player never gets bigger than 90% of the screen
	// Whilst also allowing it to be smaller based on the aspect ratio
	// If the aspect ratio wants the height to be bigger than 90% of the screen,
	// The height will be set to 90% of the screen
	// If the aspect ration wants height to be smaller then 90% of the screen,
	// The height will be set to the aspect ratio
	.video-player-container {
		display: grid;
		grid-template-columns: auto;
		grid-template-rows: auto;
		max-height: 90%;

		.aspect-ratio {
			grid-area: 1 / 1 / 1 / 1;
			aspect-ratio: 16 / 9;
			width: 100%;
			max-height: 90%;
			min-height: 100%;
		}

		.video-player {
			grid-area: 1 / 1 / 1 / 1;
			max-height: 90%;
			min-height: 100%;
			width: 100%;
		}
	}

	.channel-name {
		display: block;
		font-size: 1.75rem;
		line-height: 1;
		font-weight: 300;
		grid-row: 1 / 1;
		grid-column: 1 / 1;
	}

	.category {
		display: block;
		font-size: 1.25rem;
		color: $primaryColor;
		font-weight: 500;
		grid-row: 2 / 2;
		grid-column: 1 / 1;
	}

	.viewer-count {
		color: $primaryColor;
		> svg {
			vertical-align: -0.15em;
		}
	}

	.right-side {
		display: flex;
		align-items: center;
		grid-row: 1 / -1;
		grid-column: 2 / 2;
		justify-content: flex-end;
		gap: 1rem;
		font-size: 1.15rem;
	}

	.under-player {
		padding: 1rem 1rem 1.5rem 1rem;
		display: grid;
		grid-template-columns: 1fr auto;
		grid-template-rows: auto auto;
		border-bottom: 1px solid $borderColor;
		height: 5rem;
	}

	.follow-button {
		background-color: #3f3f3f6a;
		border: none;
		border-radius: 50%;
		padding: 0.5rem;
		color: $primaryColor;
		font-size: 1.15em;
		cursor: pointer;
		transition: background-color 0.2s ease-in-out;
		&:hover {
			background-color: #3f3f3f;
		}
	}

	.subscribe-button {
		border: none;
		border-radius: 1.5em;
		padding: 0.5rem 1rem;
		font: inherit;
		font-weight: 500;
		cursor: pointer;
		box-shadow: 0px 6px 20px 7px rgba(255, 115, 87, 0.1);
		background-color: $primaryColor;
		color: $textColor;
		transition:
			background-color 0.5s,
			color 0.5s,
			box-shadow 0.5s;
		&:hover {
			background-color: $primaryColorLight;
			box-shadow: 0px 6px 20px 7px rgba(255, 115, 87, 0.2);
		}
	}

	.super-under {
		height: 200vh;
	}
</style>
