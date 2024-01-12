<script lang="ts">
	import UserBanner from "$/components/settings/user-banner.svelte";
	import TabSelector from "$/components/tab-selector.svelte";
	import { authDialog, AuthMode, user, sessionToken } from "$/store/auth";
	import { PUBLIC_BASE_URL } from "$env/static/public";
	import { faArrowUpRightFromSquare, faRoadBarrier } from "@fortawesome/free-solid-svg-icons";
	import Fa from "svelte-fa";

	$: tabs = [
		{ name: "Profile", pathname: "/settings/profile" },
		{ name: "Account", pathname: "/settings/account", showWarning: !!$user?.channel.live },
		{ name: "Notifications", pathname: "/settings/notifications" },
		{ name: "Billing", pathname: "/settings/billing", showWarning: !!$user?.channel.live },
	];

	$: if ($sessionToken === null) {
		$authDialog = {
			opened: true,
			mode: AuthMode.Login,
		};
	}
</script>

<svelte:head>
	<title>Scuffle - Settings</title>

	<!-- Open Graph -->
	<meta property="og:title" content="Scuffle - Settings" />
	<meta property="og:description" content="Scuffle - open-source live-streaming platform" />
	<meta property="og:image" content="{PUBLIC_BASE_URL}/banner.jpeg" />
	<meta property="og:image:alt" content="Scuffle Banner" />
</svelte:head>

<div class="content">
	{#if $user}
		<div class="header">
			<h1>Settings</h1>
			<TabSelector {tabs} />
			<a class="button primary open-dashboard" href="/creator-dashboard" target="_blank">
				<Fa icon={faArrowUpRightFromSquare} />
				Open Creator Dashboard
			</a>
		</div>
		<UserBanner />
		<slot />
	{:else}
		<div class="error">
			<Fa icon={faRoadBarrier} size="3x" />
			<span>Please sign in to access the settings.</span>
		</div>
	{/if}
</div>

<style lang="scss">
	@import "../../assets/styles/variables.scss";

	.content {
		grid-area: content;
		overflow-y: auto;

		padding: 1rem;

		display: flex;
		flex-direction: column;
		gap: 2rem;

		.header {
			display: flex;
			justify-content: space-between;
			align-items: center;
			flex-wrap: wrap;
			gap: 1rem;

			& > h1 {
				line-height: 0.9;
				font-weight: 500;
				font-size: 2.25rem;
			}

			& > .open-dashboard {
				font-weight: 500;
				padding: 0.5rem 1rem;

				display: flex;
				align-items: center;
				gap: 0.5rem;
			}
		}
	}

	.error {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 0.5rem;

		height: 100%;

		font-size: 1.5rem;
	}
</style>
