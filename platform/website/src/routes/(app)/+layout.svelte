<script lang="ts">
	import TopNav from "$components/top-nav.svelte";
	import { authDialog, currentTwoFaRequest } from "$/store/auth";
	import AuthDialog from "$/components/auth/auth-dialog.svelte";
	import "$/lib/auth";
	import SideNav from "$components/side-nav.svelte";
	import { onMount } from "svelte";
	import DevBanner from "$/components/dev-banner.svelte";
	import SolveTwoFaDialog from "$/components/auth/solve-two-fa-dialog.svelte";

	let based = false;

	onMount(() => {
		// Show BASED between 10 and 60 minutes
		let delay = Math.floor(Math.random() * 50 * 60 * 1000) + 10 * 60 * 1000;
		setTimeout(() => {
			based = true;
		}, delay);
	});
</script>

<header>
	<a href="#main" class="skip-to-main">Skip to main content</a>
	<div class="top-nav">
		<DevBanner />
		<TopNav />
	</div>
	<SideNav />
</header>

<main id="main">
	<slot />

	{#if $authDialog.opened}
		<AuthDialog />
	{/if}

	{#if $currentTwoFaRequest}
		<SolveTwoFaDialog requestId={$currentTwoFaRequest} />
	{/if}

	<img class="based" src="/BASED.webp" alt="BASED" class:animate={based} />

	<footer />
</main>

<style lang="scss">
	@import "../../assets/styles/variables.scss";

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
