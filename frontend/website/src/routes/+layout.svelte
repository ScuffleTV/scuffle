<script>
	import "@fontsource/be-vietnam-pro";
	import "@fontsource/comfortaa";
	import "$assets/styles/global.scss";
	import Nav from "../components/nav.svelte";
	import { loginMode } from "../store/login";
	import Login from "../components/login.svelte";
	import { setContextClient } from "@urql/svelte";
	import { client } from "../lib/gql";
	import "../lib/user";

	// This provides the GraphQL client to all components in the app.
	setContextClient(client);
</script>

<div class="body">
	<header>
		<Nav />
	</header>

	<main>
		<div class="no-overflow">
			<slot />
		</div>
	</main>

	{#if $loginMode}
		<Login />
	{/if}

	<footer />
</div>

<style lang="scss">
	header {
		position: sticky;
		width: 100%;
		top: 0;
		z-index: 2;
	}

	main {
		min-height: 100%;
	}

	footer {
		display: grid;
		place-items: center;
	}

	.no-overflow {
		overflow: hidden;
	}

	.body {
		display: grid;
		grid-template-rows: auto 1fr auto;
		height: 100vh;
	}
</style>
