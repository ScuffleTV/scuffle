<script lang="ts">
	import UserBanner from "$/components/settings/user-banner.svelte";
	import TabSelector from "$/components/tab-selector.svelte";
	import { authDialog, AuthDialog, user, sessionToken } from "$/store/auth";
	import { faArrowUpRightFromSquare, faRoadBarrier } from "@fortawesome/free-solid-svg-icons";
	import Fa from "svelte-fa";

	$: isLive = typeof $user?.channel.liveViewerCount === "number";

	$: tabs = [
		{ name: "Profile", pathname: "/settings/profile" },
		{ name: "Account", pathname: "/settings/account", showWarning: isLive },
		{ name: "Notifications", pathname: "/settings/notifications" },
		{ name: "Billing", pathname: "/settings/billing", showWarning: isLive },
	];

	$: if ($sessionToken === null) {
		$authDialog = AuthDialog.Login;
	}
</script>

<svelte:head>
	<title>Scuffle - Settings</title>
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
		<!-- TODO: Improve error -->
		<div class="error">
			<Fa icon={faRoadBarrier} size="3x" />
			<span>Please log in to view this page.</span>
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
