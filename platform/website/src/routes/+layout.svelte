<script lang="ts">
	import "$assets/styles/global.scss";
	import TopNav from "$components/top-nav.svelte";
	import { authDialog, currentTwoFaRequest } from "$/store/auth";
	import AuthDialog from "$/components/auth/auth-dialog.svelte";
	import { setContextClient } from "@urql/svelte";
	import "$/lib/auth";
	import SideNav from "$components/side-nav.svelte";
	import { onMount } from "svelte";
	import type { LayoutData } from "./$types";
	import { building, dev } from "$app/environment";
	import Spinner from "$/components/spinner.svelte";
	import DevBanner from "$/components/dev-banner.svelte";
	import SolveTwoFaDialog from "$/components/auth/solve-two-fa-dialog.svelte";
	import { PUBLIC_TWITTER_HANDLE } from "$env/static/public";

	export let data: LayoutData;

	setContextClient(data.client);

	let based = false;

	for (const el of document.querySelectorAll(".remove-after-load")) {
		el.remove();
	}

	onMount(() => {
		// Show BASED between 10 and 60 minutes
		let delay = Math.floor(Math.random() * 50 * 60 * 1000) + 10 * 60 * 1000;
		setTimeout(() => {
			based = true;
		}, delay);
	});
</script>

<svelte:head>
	<meta name="description" content="Scuffle - open-source live-streaming platform" />
	<meta name="keywords" content="scuffle, live, stream, watch, live-streaming, open-source" />

	<!-- Open Graph -->
	<meta property="og:type" content="website" />
	<meta property="og:site_name" content="Scuffle" />
	<!-- TODO: Change this when localizing -->
	<meta property="og:locale" content="en_US" />

	<!-- Twitter -->
	<meta name="twitter:card" content="summary" />
	<meta name="twitter:title" content="Scuffle" />
	<meta name="twitter:description" content="Scuffle - open-source live-streaming platform" />
	<meta name="twitter:site" content={PUBLIC_TWITTER_HANDLE} />
</svelte:head>

<header>
	<a href="#main" class="skip-to-main">Skip to main content</a>
	<div class="top-nav">
		<DevBanner />
		<TopNav />
	</div>
	{#if !building}
		<SideNav />
	{/if}
</header>

<main id="main">
	{#if building}
		<Spinner />
	{:else}
		<slot />

		{#if $authDialog}
			<AuthDialog />
		{/if}

		{#if $currentTwoFaRequest}
			<SolveTwoFaDialog requestId={$currentTwoFaRequest} />
		{/if}

		<img class="based" src="/BASED.webp" alt="BASED" class:animate={based} />

		<footer />
	{/if}
</main>

<style lang="scss">
	@import "../assets/styles/variables.scss";

	header,
	main {
		display: contents;
	}

	.skip-to-main {
		position: absolute;
		color: $primaryColor;
		text-decoration: none;
		opacity: 0;
		pointer-events: none;

		&:focus-visible {
			text-decoration: underline;
			opacity: 1;
			pointer-events: unset;
		}
	}

	.top-nav {
		grid-area: top-nav;
	}

	.based {
		position: absolute;
		bottom: 0;
		left: 0;

		filter: saturate(0) opacity(0.2);
		display: none;

		&.animate {
			display: block;
			animation: based 10s ease-out both;
		}
	}

	@keyframes based {
		from {
			transform: translateX(-100%);
		}
		50% {
			transform: translateX(-70%);
		}
		to {
			transform: translateX(-2000%);
		}
	}
</style>
