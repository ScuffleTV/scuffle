<script>
	import "@fontsource/be-vietnam-pro";
	import "@fontsource/comfortaa";
	import "$assets/styles/global.scss";
	import Nav from "../components/nav.svelte";
	import { loginMode } from "../store/login";
	import Login from "../components/login.svelte";
	import { setContextClient } from "@urql/svelte";
	import { client } from "../lib/gql";

	// This provides the GraphQL client to all components in the app.
	setContextClient(client);
</script>

<div class="no-overflow">
	<header>
		<Nav />
	</header>

	<main>
		<slot />
	</main>

	{#if $loginMode}
		<Login />
	{/if}

	<footer />
</div>

<style lang="scss">
	header {
		position: fixed;
		width: 100%;
		top: 0;
		z-index: 2;
	}

	main {
		overflow: visible;
		min-height: 95vh;
	}

	footer {
		display: grid;
		place-items: center;
	}

	.no-overflow {
		position: relative;
		overflow: hidden;
	}
</style>
