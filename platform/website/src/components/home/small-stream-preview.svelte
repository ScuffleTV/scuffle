<script lang="ts">
	import type { User } from "$/gql/graphql";
	import { isHover, viewersToString } from "$/lib/utils";
	import Player from "../player.svelte";

	// export let streamer: string;
	// export let title: string;
	// export let viewers: number;
	export let avatar: string;
	export let preview: string;

	export let user: User;

	$: viewers = viewersToString(user.channel.liveViewerCount ?? 0, true);

	let timeout: number | NodeJS.Timeout;
	let focused: boolean = false;

	function onFocus() {
		if (!isHover()) return;
		if (focused) return;
		timeout = setTimeout(() => {
			focused = true;
		}, 1000);
	}

	function onBlur() {
		clearTimeout(timeout);
		focused = false;
	}
</script>

<a
	class="preview"
	href="/{user.username}"
	on:mouseenter={onFocus}
	on:mouseleave={onBlur}
	on:focus={onFocus}
	on:blur={onBlur}
	aria-label="{user.displayName} streaming {user.channel.title} with {viewers}"
>
	{#if focused}
		<div class="video">
			<Player roomId={user.channel.roomId} controls={false} initMuted />
		</div>
	{:else}
		<img src={preview} alt="Stream Thumbnail" class="thumbnail" />
	{/if}
	<img src={avatar} alt="{user.displayName}'s avatar" class="avatar" />
	<div class="text-container">
		<span class="title">{user.channel.title}</span>
		<span class="name">{user.displayName}</span>
	</div>
	<span class="viewers">{viewers}</span>
</a>

<style lang="scss">
	@import "../../assets/styles/variables.scss";

	.preview {
		position: relative;

		color: $textColor;
		text-decoration: none;

		display: grid;
		grid-template:
			"preview preview"
			"avatar text"
			"avatar text";
		grid-template-rows: auto auto auto;
		grid-template-columns: auto 1fr;
		gap: 0.5rem;

		border-radius: 0.5rem;
		padding: 0.5rem;
		transition: background-color 0.2s;

		&:hover {
			background-color: $bgColorLight;
		}
	}

	.viewers {
		position: absolute;

		top: 1rem;
		left: 0.5rem;

		font-size: 0.875rem;
		background-color: rgba(0, 0, 0, 0.5);
		padding: 0.25rem 0.4rem;
		border-radius: 0 0.25rem 0.25rem 0;
	}

	.thumbnail,
	.video {
		grid-area: preview;

		width: 100%;
		aspect-ratio: 16 / 9;
	}

	.avatar {
		grid-area: avatar;

		width: 2.5rem;
		height: 2.5rem;
		border-radius: 50%;
	}

	.text-container {
		grid-area: text;

		display: flex;
		flex-direction: column;
		overflow: hidden;

		& > .title {
			font-weight: 500;
			white-space: nowrap;
			overflow: hidden;
			text-overflow: ellipsis;
		}

		& > .name {
			color: $textColorLight;
			font-size: 0.875rem;
			white-space: nowrap;
			overflow: hidden;
			text-overflow: ellipsis;
		}
	}
</style>
