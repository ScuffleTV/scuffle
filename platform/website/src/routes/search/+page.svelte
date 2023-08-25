<script lang="ts">
	import type { PageData } from "./$types";

	export let data: PageData;
</script>

<svelte:head>
	<title>Scuffle - Search Results</title>
</svelte:head>

<h1>Search results for {data.query}</h1>
