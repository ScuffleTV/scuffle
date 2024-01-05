<script lang="ts">
	import { viewersToString } from "$lib/utils";
	import { page } from "$app/stores";
	import type { Channel, DisplayColor } from "$/gql/graphql";
	import DefaultAvatar from "../user/default-avatar.svelte";
	import { fade } from "svelte/transition";

	export let id: string;
	export let username: string;
	export let displayName: string;
	export let displayColor: DisplayColor;
	// typescript pain
	export let channel: {
		live?:
			| {
					liveViewerCount: number;
			  }
			| null
			| undefined;
		category?:
			| {
					name: string;
			  }
			| null
			| undefined;
	};
	export let collapsed = false;

	$: ariaLabel = channel.live
		? `${displayName} streaming ${channel.category?.name ?? ""} with ${viewersToString(
				channel.live.liveViewerCount,
				true,
			)}`
		: `${displayName} is offline`;

	$: selected =
		$page.url.pathname === `/${username}` || $page.url.pathname.startsWith(`/${username}/`);
</script>

<a
	class="streamer-card"
	href="/{username}"
	class:selected
	class:collapsed
	aria-label={ariaLabel}
	in:fade={{ duration: 200 }}
>
	<div class="avatar">
		<DefaultAvatar userId={id} {displayColor} size={2 * 16} />
	</div>
	{#if !collapsed}
		<div class="text-container">
			<span class="name" class:offline={!channel.live}>{displayName}</span>
			{#if channel.live && channel.category}
				<span class="category">{channel.category.name}</span>
			{/if}
		</div>
		<span
			class="viewers"
			aria-label={channel.live ? viewersToString(channel.live.liveViewerCount, true) : "offline"}
			class:online={channel.live}
		>
			{channel.live ? viewersToString(channel.live.liveViewerCount) : "Offline"}
		</span>
	{/if}
</a>

<style lang="scss">
	@import "../../assets/styles/variables.scss";

	.streamer-card {
		display: flex;
		gap: 0.5rem;
		align-items: center;

		padding: 0.5rem 0.75rem;
		padding-left: 0.625rem;
		color: $textColor;
		text-decoration: none;
		border-left: 0.125rem solid transparent;

		&:hover,
		&:focus-visible {
			background-color: $bgColorLight;
		}

		&.selected {
			border-color: $primaryColor;
		}
	}

	.avatar {
		justify-self: center;

		display: flex;
	}

	.text-container {
		flex-grow: 1;

		display: flex;
		flex-direction: column;
		justify-content: center;
	}

	.name {
		color: $textColor;
		font-weight: 500;
		font-size: 1rem;

		&.offline {
			grid-row: 1 / span 2;
		}
	}

	.category {
		color: $textColorLight;
		font-weight: 500;
		font-size: 0.865rem;

		/* if the category name is too long, we want to cut it off and add an ellipsis */
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.viewers {
		justify-self: end;
		align-self: flex-start;
		color: $textColorLighter;
		font-weight: 500;
		font-size: 0.865rem;

		white-space: nowrap;
	}

	// We need to make a red dot appear on the avatar when the streamer is live.
	.viewers.online::before {
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
