<script lang="ts">
	import "$assets/styles/global.scss";
	import { PUBLIC_TWITTER_HANDLE } from "$env/static/public";
	import { onMount } from "svelte";
	import type { LayoutData } from "./$types";
	import { setContextClient } from "@urql/svelte";

	export let data: LayoutData;

	setContextClient(data.client);

	onMount(() => {
		for (const el of document.querySelectorAll(".remove-after-load")) {
			el.remove();
		}
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
	<meta name="twitter:site" content="@{PUBLIC_TWITTER_HANDLE}" />
</svelte:head>

<slot />
