<script lang="ts">
	import { PUBLIC_BASE_URL } from "$env/static/public";
	import CategoryCard from "$/components/home/category-card.svelte";
	import SmallStreamPreview from "$/components/home/small-stream-preview.svelte";
	import User from "$/components/search/user.svelte";
	import type { PageData } from "./$types";
	import type { Category, SearchAllResults, SearchResult, User as UserData } from "$/gql/graphql";
	import { searchQuery } from "$/lib/search";
	import { getContextClient } from "@urql/svelte";
	import ShowMore from "$/components/show-more.svelte";
	import Spinner from "$/components/spinner.svelte";
	import Sadge from "$/components/sadge.svelte";

	const INIT_LIMIT = 10;

	let limit = INIT_LIMIT;
	let offset = 0;

	export let data: PageData;

	let results: SearchAllResults | undefined = undefined;
	$: totalCount = results?.totalCount;
	$: liveUserResults = results?.results
		.filter((r) => r.object.__typename === "User" && r.object.channel.live)
		.map((r) => r.object as UserData);
	$: offlineUserResults = results?.results
		.filter((r) => r.object.__typename === "User" && !r.object.channel.live)
		.map((r) => r.object as UserData);
	$: categoryResults = results?.results
		.filter((r) => r.object.__typename === "Category")
		.map((r) => r.object as Category);

	const client = getContextClient();

	$: if (data.query) {
		results = undefined;
		searchQuery(client, data.query, INIT_LIMIT).then((result) => {
			results = result.data?.resp as SearchAllResults;
		});
	}

	function loadMore() {
		limit += INIT_LIMIT;
		offset += INIT_LIMIT;
		searchQuery(client, data.query, limit, offset).then((result) => {
			if (result.data?.resp && results) {
				results.totalCount = result.data.resp.totalCount;
				results.results = [...results.results, ...(result.data.resp.results as SearchResult[])];
			}
		});
	}
</script>

<svelte:head>
	<title>Scuffle - Search Results</title>

	<!-- Open Graph -->
	<meta property="og:title" content="Scuffle - Search Results" />
	<meta property="og:description" content="Scuffle - open-source live-streaming platform" />
	<meta property="og:image" content="{PUBLIC_BASE_URL}/banner.jpeg" />
	<meta property="og:image:alt" content="Scuffle Banner" />
</svelte:head>

{#if results && results.results.length > 0}
	<div class="content results">
		<h2 class="num-results">{totalCount} results for “{data.query}”</h2>
		{#if liveUserResults?.length && liveUserResults?.length > 0}
			<div class="container">
				<h1>Live Channels</h1>
				<section class="video-preview">
					{#each liveUserResults as user}
						<article>
							<SmallStreamPreview
								{user}
								avatar="https://static-cdn.jtvnw.net/jtv_user_pictures/xqc-profile_image-9298dca608632101-300x300.jpeg"
								preview="/xqc-preview.png"
							/>
						</article>
					{/each}
				</section>
			</div>
		{/if}
		{#if offlineUserResults?.length && offlineUserResults?.length > 0}
			<div class="container">
				<h1>Offline Channels</h1>
				<section class="offline-channels">
					{#each offlineUserResults as user}
						<article>
							<User {user} grayWhenOffline={false} />
						</article>
					{/each}
				</section>
			</div>
		{/if}
		{#if categoryResults?.length && categoryResults?.length > 0}
			<div class="container">
				<h1>Categories</h1>
				<section class="categories">
					{#each categoryResults as category}
						<article>
							<CategoryCard title={category.name} image="/categories/minecraft.png" viewers={420} />
						</article>
					{/each}
				</section>
			</div>
		{/if}
		{#if totalCount && totalCount > limit}
			<ShowMore on:click={loadMore} />
		{/if}
	</div>
{:else if results}
	<div class="content center">
		<Sadge />
		<span class="no-results">No results for “{data.query}”</span>
	</div>
{:else}
	<div class="content center">
		<Spinner />
	</div>
{/if}

<style lang="scss">
	@import "../../../assets/styles/variables.scss";

	.no-results {
		color: $textColorLight;
		font-size: 1.2rem;
		font-weight: 400;
	}

	.content {
		grid-area: content;
		overflow-y: auto;
		padding: 1rem;

		&.results {
			display: flex;
			flex-direction: column;
			gap: 2rem;
		}

		&.center {
			display: flex;
			flex-direction: column;
			justify-content: center;
			align-items: center;
			gap: 1rem;
		}
	}

	.num-results {
		color: $textColorLight;
		font-size: 1.2rem;
		font-weight: 400;
	}

	section.video-preview {
		display: grid;
		gap: 2rem;
		grid-template-columns: repeat(auto-fill, minmax(18rem, 1fr));
	}

	section.offline-channels {
		display: flex;
		gap: 1rem;
	}

	section.categories {
		display: flex;
		flex-wrap: wrap;
		gap: 2rem;

		& > article {
			width: 9.5rem;
		}
	}

	h1 {
		margin-bottom: 1rem;
	}
</style>
