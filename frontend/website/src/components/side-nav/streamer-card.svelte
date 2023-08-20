<script lang="ts">
	import { viewersToString } from "$lib/utils";
	import { page } from "$app/stores";

	export let displayName: string;
	export let username: string;
	export let avatar: string;
	export let game: string;
	export let viewers: number | null;
</script>

<a
	class="streamer-card"
	href={`/${username}`}
	class:selected={$page.url.pathname === `/${username}`}
	aria-label={`${displayName} streaming ${game} with ${viewersToString(viewers, true)}`}
>
	<img class="avatar" src={avatar} alt="User avatar" class:offline={viewers === null} />
	<span class="name">{displayName}</span>
	<span class="game">{game}</span>
	<span
		class="viewers"
		aria-label={viewers ? viewersToString(viewers, true) : "offline"}
		class:offline={viewers === null}>{viewersToString(viewers)}</span
	>
</a>

<style lang="scss">
	@import "../../assets/styles/variables.scss";

	.streamer-card {
		display: grid;
		column-gap: 0.5rem;

		padding: 0.5rem 0.75rem;
		padding-left: 0.625rem;
		color: $textColor;
		font-family: $sansFont;
		text-decoration: none;
		border-left: 0.125rem solid transparent;

		grid-template-rows: 1fr 1fr;
		grid-template-columns: auto 1fr auto;
		grid-template-areas:
			"avatar name viewers"
			"avatar game .";

		&:hover,
		&:focus-visible {
			background-color: $bgColorLight;
		}

		&.selected {
			border-color: $primaryColor;
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
		color: $textColor;
		font-weight: 500;
		font-size: 1rem;
	}

	.game {
		grid-area: game;
		align-self: center;
		color: $textColorLight;
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
		color: $textColorLighter;
		font-weight: 500;
		font-size: 0.865rem;
	}

	// We need to make a red dot appear on the avatar when the streamer is live.
	.viewers:not(.offline)::before {
		content: "";
		display: inline-block;
		width: 0.4rem;
		height: 0.4rem;
		background-color: $liveColor;
		border-radius: 50%;
		margin-right: 0.4rem;
		margin-bottom: 0.1rem;
	}
</style>
