<script lang="ts">
	import "$assets/styles/global.scss";
	import TopNav from "../components/topNav.svelte";
	import { loginMode } from "../store/login";
	import Login from "../components/login.svelte";
	import { setContextClient } from "@urql/svelte";
	import { client } from "../lib/gql";
	import "../lib/user";
	import SideNav from "../components/sideNav.svelte";

	// This provides the GraphQL client to all components in the app.
	setContextClient(client);
</script>

<main>
	<SideNav />
	<TopNav />
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
		grid-template-rows: auto 1fr auto;
		grid-template-columns: auto 1fr auto;
		min-height: 100%;
	}
</style>
