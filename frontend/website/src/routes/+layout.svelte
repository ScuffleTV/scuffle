<script lang="ts">
	import "$assets/styles/global.scss";
	import TopNav from "$components/top-nav.svelte";
	import { loginMode } from "$store/login";
	import Login from "$components/login.svelte";
	import { setContextClient } from "@urql/svelte";
	import { client } from "$lib/gql";
	import "$lib/user";
	import SideNav from "$components/side-nav.svelte";

	// This provides the GraphQL client to all components in the app.
	setContextClient(client);
</script>

<main>
	<TopNav />
	<SideNav />

	<slot />

	{#if $loginMode}
		<Login />
	{/if}

	<footer />
</main>

<style lang="scss">
	@import "../assets/styles/variables.scss";

	main {
		display: grid;
		grid-template-areas:
			"top-nav top-nav"
			"side-nav content";
		grid-template-rows: auto 1fr;
		grid-template-columns: auto 1fr;
		min-height: 100vh;
	}
</style>
