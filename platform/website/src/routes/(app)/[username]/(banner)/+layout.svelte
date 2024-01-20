<script lang="ts">
	import Chatroom from "$/components/chat/chatroom.svelte";
	import type { LayoutData } from "./$types";
	import TabSelector from "$/components/tab-selector.svelte";
	import DisplayName from "$/components/user/display-name.svelte";
	import { followersToString } from "$/lib/utils";
	import FollowButton from "$/components/user/follow-button.svelte";
	import SubscribeButton from "$/components/user/subscribe-button.svelte";
	import BrandIcon from "$/components/icons/brand-icon.svelte";
	import { userId } from "$/store/auth";
	import ProfilePicture from "$/components/user/profile-picture.svelte";
	import OfflineBanner from "$/components/channel/offline-banner.svelte";

	export let data: LayoutData;
	$: channelId = data.user.id;

	$: offlineTabs = [
		{
			name: "Home",
			pathname: `/${data.user.username}/home`,
		},
		{
			name: "Videos",
			pathname: `/${data.user.username}/videos`,
		},
		{
			name: "Clips",
			pathname: `/${data.user.username}/clips`,
		},
		{
			name: "About",
			pathname: `/${data.user.username}/about`,
		},
	];

	let chatCollapsed = true;
</script>

<div class="content">
	<div class="offline-page">
		<!-- 100vw width on mobile, calc(100vw - sidebar) width on desktop -->
		<OfflineBanner
			channelId={data.user.id}
			bind:offlineBanner={data.user.channel.offlineBanner}
			fullPageWidth="(max-width: 768px) 100vw, calc(100vw - 16rem)"
		>
			<div class="user-card">
				<div class="user-info">
					<!-- Wrapper div -->
					<div class="avatar">
						<ProfilePicture
							userId={channelId}
							bind:profilePicture={data.user.profilePicture}
							bind:displayColor={data.user.displayColor}
							size={3.5 * 16}
						/>
					</div>
					<h1 class="name">
						<DisplayName userId={channelId} bind:displayName={data.user.displayName} />
					</h1>
					<span class="followers">{followersToString(data.user.channel.followersCount)}</span>
				</div>
				{#if data.user.channel.description}
					<span class="description">{data.user.channel.description}</span>
				{/if}
				{#if data.user.channel.links.length > 0}
					<ul class="socials">
						{#each data.user.channel.links as link}
							<li>
								<a href={link.url} target="_blank">
									<BrandIcon url={link.url} />
									<span>{link.name}</span>
								</a>
							</li>
						{/each}
					</ul>
				{/if}
			</div>
		</OfflineBanner>
		<div class="page" class:hide-on-mobile={!chatCollapsed}>
			<div class="row">
				<TabSelector tabs={offlineTabs} />
				<div class="buttons">
					{#if $userId !== channelId}
						<FollowButton {channelId} bind:following={data.following} />
						<SubscribeButton />
					{/if}
				</div>
			</div>
			<slot />
		</div>
	</div>
	<Chatroom {channelId} bind:collapsed={chatCollapsed} />
</div>

<style lang="scss">
	@import "../../../../assets/styles/variables.scss";

	.content {
		grid-area: content;

		display: flex;

		overflow-y: auto;

		& > .offline-page {
			overflow-y: auto;

			flex-grow: 1;
		}
	}

	.user-card {
		background-color: $bgColor;
		padding: 1rem;
		border-radius: 0.5rem;
		border: 2px solid $borderColor;
		margin: 1rem;

		max-width: 20rem;

		display: flex;
		flex-direction: column;
		gap: 1rem;

		& > .user-info {
			display: grid;
			grid-template-areas: "avatar name" "avatar followers";
			justify-content: start;
			column-gap: 0.5rem;
			row-gap: 0.25rem;
			grid-template-rows: 1fr 1fr;

			& > .avatar {
				grid-area: avatar;
			}

			& > .name {
				grid-area: name;
				align-self: end;

				font-size: 1.5rem;
				font-weight: 600;
				line-height: 0.9em;

				color: $textColor;
			}

			& > .followers {
				grid-area: followers;
				align-self: start;

				font-weight: 500;
				color: $textColorLight;
			}
		}

		& > .description {
			color: $textColorLighter;
			text-wrap: wrap;
		}

		& > .socials {
			list-style: none;
			margin: 0;
			padding: 0;

			& > li {
				padding: 0.15rem 0;

				& > a {
					color: $textColor;
					text-decoration: none;
					font-weight: 500;

					&:hover,
					&:focus-visible {
						& > span {
							text-decoration: underline;
						}
					}
				}
			}
		}
	}

	.page {
		padding: 1rem;

		& > .row {
			display: flex;
			justify-content: space-between;
			align-items: center;
			gap: 1rem;
			flex-wrap: wrap-reverse;

			& > .buttons {
				flex-grow: 1;
				display: flex;
				gap: 1rem;
				justify-content: flex-end;
			}
		}
	}

	@media screen and (max-width: $mobileBreakpoint) {
		.content {
			flex-direction: column;

			& > .offline-page {
				flex-grow: 0;
			}
		}
	}
</style>
