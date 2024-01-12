<script lang="ts">
	import type { User } from "$/gql/graphql";
	import { viewersToString } from "$/lib/utils";
	import { createEventDispatcher } from "svelte";
	import DefaultAvatar from "../user/default-avatar.svelte";
	import ProfilePicture from "../user/profile-picture.svelte";

	const dispatch = createEventDispatcher();

	export let user: User;
	export let grayWhenOffline = true;
</script>

<a on:click={() => dispatch("close")} href="/{user.username}">
	<div class="avatar">
		<ProfilePicture
			userId={user.id}
			profilePicture={user.profilePicture}
			displayColor={user.displayColor}
			size={2.5 * 16}
		/>
	</div>
	<div class="text-container">
		<span class="name ellipsis">
			<span class:offline={grayWhenOffline && !user.channel.live}>{user.displayName}</span>
			{#if user.channel.live && user.channel.category?.name}
				<span class="category">• {user.channel.category.name}</span>
			{/if}
		</span>
		{#if user.channel.live && user.channel.title}
			<span class="title ellipsis">{user.channel.title}</span>
		{/if}
	</div>
	{#if user.channel.live}
		<span class="live-viewers">{viewersToString(user.channel.live.liveViewerCount)}</span>
	{/if}
</a>

<style lang="scss">
	@import "../../assets/styles/search-result.scss";

	.title,
	.category {
		color: $textColorLight;
		font-size: 0.865rem;
		font-weight: 500;
	}

	.live-viewers {
		font-size: 0.865rem;
		font-weight: 500;

		margin-right: 0.2rem;

		white-space: nowrap;

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

	.offline {
		color: $textColorLight;
	}
</style>
