<script lang="ts">
	import { PUBLIC_ASSET_BASE_URL } from "$env/static/public";
	import type { PageData } from "./$types";

	export let data: PageData;
</script>

<svelte:head>
	<title>Scuffle - Search Results</title>

	<!-- Open Graph -->
	<meta property="og:title" content="Scuffle - Search Results" />
	<meta property="og:description" content="Scuffle - open-source live-streaming platform" />
	<meta property="og:image" content="{PUBLIC_ASSET_BASE_URL}/banner.jpeg" />
	<meta property="og:image:alt" content="Scuffle Banner" />
</svelte:head>

<h1>Search results for {data.query}</h1>
