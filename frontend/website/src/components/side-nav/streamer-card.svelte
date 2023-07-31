<script lang="ts">
	import { viewersToString } from "$lib/utils";

	export let name: string;
	export let avatar: string;
	export let game: string;
	export let viewers: number | null;

	// We want to convert number of viewers to a string with commas.
	// For example, 1000 should become 1K.
	// 1300 should become 1,3K.
	// Anything bigger than 100 000 should become 100K without any decimals.
	// 1 000 000 should become 1M.
	// 1 300 000 should become 1,3M.

	// In svelte the $ prefix is used to create a reactive variable.
	// This means that if the value of viewers changes, viewersString will be updated.
	$: viewersString = viewersToString(viewers);
</script>

<a class="streamer-card" href={`/${name.toLowerCase()}`}>
	<img class="avatar" src={avatar} alt="Streamer avatar" class:offline={viewers === null} />
	<div class="name">{name}</div>
	<div class="game">{game}</div>
	<div class="viewers" class:offline={viewers === null}>{viewersString}</div>
</a>

<style lang="scss">
	.streamer-card {
		display: grid;
		column-gap: 0.5rem;
		padding: 0.5rem 0.75rem;
		color: white;
		font-family: "Inter", sans-serif;
		text-decoration: none;
		grid-template-rows: 1fr 1fr;
		grid-template-columns: auto 1fr auto;
		grid-template-areas:
			"avatar name viewers"
			"avatar game .";

		&:hover {
			background-color: #252525;
		}
	}

	.avatar {
		grid-area: avatar;
		height: 2rem;
		aspect-ratio: 1/1;
		border-radius: 50%;
		place-self: center;
		&.offline {
			filter: grayscale(100%);
		}
	}

	.name {
		grid-area: name;
		align-self: center;
		color: white;
		font-weight: 500;
		font-size: 1rem;
	}

	.game {
		grid-area: game;
		align-self: center;
		color: #a3a3a3;
		font-weight: 500;
		font-size: 0.865rem;

		/* if the game is too long, we want to cut it off and add an ellipsis */
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.viewers {
		grid-area: viewers;
		align-self: center;
		justify-self: end;
		color: #cbcbcb;
		font-weight: 500;
		font-size: 0.865rem;
	}

	// We need to make a red dot appear on the avatar when the streamer is live.
	.viewers:not(.offline)::before {
		content: "";
		display: inline-block;
		width: 0.4rem;
		height: 0.4rem;
		background-color: #e91916;
		border-radius: 50%;
		margin-right: 0.4rem;
		margin-bottom: 0.1rem;
	}
</style>
