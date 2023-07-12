<script lang="ts">
	import "$assets/styles/global.scss";
	import TopNav from "$components/top-nav.svelte";
	import { loginMode } from "$store/login";
	import Login from "$components/login.svelte";
	import { setContextClient } from "@urql/svelte";
	import { client } from "$lib/gql";
	import "$lib/user";
	import SideNav from "$components/side-nav.svelte";
	import { onMount } from "svelte";

	// This provides the GraphQL client to all components in the app.
	setContextClient(client);

	let based = false;

	onMount(() => {
		// Show BASED between 10 and 60 minutes
		let delay = Math.floor(Math.random() * 50 * 60 * 1000) + 10 * 60 * 1000;
		setTimeout(() => {
			based = true;
		}, delay);
	});
</script>

<div class="page-body">
	<header>
		<a href="#main" class="skip-to-main">Skip to main content</a>
		<TopNav />
		<SideNav />
	</header>

	<main id="main">
		<slot />

		{#if $loginMode}
			<Login />
		{/if}

		<img class="based" src="/BASED.webp" alt="BASED" class:animate={based} />
		<img class="based" src="/BASED.webp" alt="BASED" class:animate={based} />

		<footer />
		<img class="based" src="/BASED.webp" alt="BASED" class:animate={based} />

		<footer />
	</main>
</div>

<style lang="scss">
	@import "../assets/styles/variables.scss";

	.page-body {
		display: grid;
		grid-template-areas:
			"top-nav top-nav top-nav"
			"side-nav content chat";
		grid-template-rows: auto 1fr auto;
		grid-template-columns: auto 1fr auto;
		min-height: 100vh;
		max-height: 100vh;
		overflow: hidden;
	}

	header,
	main {
		display: contents;
	}

	.skip-to-main {
		position: absolute;
		color: $primaryColor;
		text-decoration: none;
		opacity: 0;

		&:focus-visible {
			text-decoration: underline;
			opacity: 1;
		}
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
