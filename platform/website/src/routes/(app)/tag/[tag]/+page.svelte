<script lang="ts">
	import { PUBLIC_BASE_URL } from "$env/static/public";
	import type { PageData } from "./$types";

	export let data: PageData;
</script>

<svelte:head>
	<title>Scuffle - Tag {data.tag}</title>

	<!-- Open Graph -->
	<meta property="og:title" content="Scuffle - Tag {data.tag}" />
	<meta property="og:description" content="Scuffle - open-source live-streaming platform" />
	<meta property="og:image" content="{PUBLIC_BASE_URL}/banner.jpeg" />
	<meta property="og:image:alt" content="Scuffle Banner" />
</svelte:head>

<div class="content">
	<span>Tag</span>
	<h1>{data.tag}</h1>
</div>

<style lang="scss">
	.content {
		grid-area: content;
	}
</style>
