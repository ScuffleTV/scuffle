<script lang="ts">
	import Chatroom from "$/components/chat/chatroom.svelte";
	import type { PageData } from "./$types";
	import Tag from "$/components/tag.svelte";
	import Player from "$/components/player.svelte";
	import { formatDuration, viewersToString } from "$/lib/utils";
	import { browser, dev } from "$app/environment";
	import DefaultAvatar from "$/components/user/default-avatar.svelte";
	import { user } from "$/store/auth";
	import { onDestroy } from "svelte";
	import DisplayName from "$/components/user/display-name.svelte";
	import FollowButton from "$/components/user/follow-button.svelte";
	import SubscribeButton from "$/components/user/subscribe-button.svelte";

	export let data: PageData;
	$: channelId = data.user.id;

	let chatCollapsed = data.chatroomCollapsed;

	$: if (browser) {
		localStorage.setItem("layout_chatroomCollapsed", JSON.stringify(chatCollapsed));
	}

	$: viewers =
		typeof data.user.channel.liveViewerCount === "number"
			? viewersToString(data.user.channel.liveViewerCount)
			: undefined;

	let timeLive =
		data.user.channel.lastLiveAt && formatDuration(new Date(data.user.channel.lastLiveAt));
	let timeInterval: NodeJS.Timeout | number;

	function setTimeLiveInterval() {
		clearInterval(timeInterval);
		timeInterval = setInterval(() => {
			if (data.user.channel.lastLiveAt) {
				timeLive = formatDuration(new Date(data.user.channel.lastLiveAt));
			}
		}, 500);
	}

	$: if (data.user.channel.lastLiveAt) {
		setTimeLiveInterval();
	}

	onDestroy(() => {
		clearInterval(timeInterval);
	});
</script>

<div class="content">
	<div class="user-container" class:dev>
		<Player {channelId} />
		<div class="under-player" class:hide-on-mobile={!chatCollapsed}>
			<div class="row title-row">
				<h1 class="title">{data.user.channel.title ?? ""}</h1>
				<div class="stream-info">
					{#if viewers}
						<span class="viewers">{viewers}</span>
					{/if}
					{#if timeLive}
						<span class="time">{timeLive}</span>
					{/if}
				</div>
			</div>
			<div class="row">
				<div>
					<div class="user">
						<!-- Wrapper div -->
						<div class="avatar">
							<DefaultAvatar
								userId={channelId}
								bind:displayColor={data.user.displayColor}
								size={40}
							/>
						</div>
						<div class="container">
							<h1 class="name">
								<DisplayName userId={channelId} bind:displayName={data.user.displayName} />
							</h1>
							{#if data.user.channel.category}
								<span class="category">{data.user.channel.category.name}</span>
							{/if}
						</div>
					</div>
					{#if $user?.id !== channelId}
						<FollowButton {channelId} bind:following={data.following} />
					{/if}
				</div>
				{#if $user?.id !== channelId}
					<SubscribeButton />
				{/if}
			</div>
			<div class="tags">
				<Tag content="English" />
				<Tag content="open source" />
				<Tag content="streaming" />
			</div>
		</div>
	</div>
	<Chatroom {channelId} bind:collapsed={chatCollapsed} />
</div>

<style lang="scss">
	@import "../../assets/styles/variables.scss";

	.content {
		grid-area: content;

		display: flex;

		& > .user-container {
			flex-grow: 1;

			// I tried very long to figure out why we need a fixed height here to make it scrollable
			// I didn't find out why yet
			max-height: calc(100svh - $topNavHeight);
			overflow-y: auto;

			&.dev {
				max-height: calc(100svh - $topNavHeight - $devBannerHeight);
			}
		}
	}

	@media screen and (max-width: $mobileBreakpoint) {
		.content {
			flex-direction: column;

			overflow-y: auto;

			& > .user-container {
				flex-grow: 0;
				overflow-y: unset;
			}
		}
	}

	.under-player {
		padding: 1rem;
		font-family: $sansFont;
		font-weight: 500;

		display: flex;
		gap: 1rem;
		flex-direction: column;

		.row {
			display: flex;
			align-items: center;
			justify-content: space-between;
			gap: 0.5rem;
			flex-wrap: wrap;

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
			display: flex;
			gap: 0.5rem;
			align-items: center;

			& > .avatar {
				grid-row: 1 / -1;
				width: 2.5rem;
				height: 2.5rem;
				border-radius: 50%;
			}

			& > .container {
				display: flex;
				flex-direction: column;
				justify-content: center;

				& > .name {
					font-size: 1.25rem;
					overflow: hidden;
					text-overflow: ellipsis;
					white-space: nowrap;
				}

				& > .category {
					font-size: 0.875rem;
					color: $textColorLight;
					overflow: hidden;
					text-overflow: ellipsis;
					white-space: nowrap;
				}
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
