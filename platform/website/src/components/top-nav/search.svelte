<script lang="ts">
	import Fa from "svelte-fa";
	import { faMagnifyingGlass } from "@fortawesome/free-solid-svg-icons";
	import { getContextClient } from "@urql/svelte";
	import MouseTrap from "../mouse-trap.svelte";
	import { goto } from "$app/navigation";
	import Category from "../search/category.svelte";
	import User from "../search/user.svelte";
	import type { SearchResult } from "$/gql/graphql";
	import { searchQuery } from "$/lib/search";

	function onSubmit(e: Event) {
		if (query) {
			clearTimeout(timeout);
			closed = true;
		} else {
			e.preventDefault();
			queryInput.focus();
		}
		if (results) {
			/// Directly navigate to the result if it's the only exact match
			const exactMatches = results.filter((r) => r.similarity === 1.0);
			if (exactMatches.length === 1) {
				const result = exactMatches[0];
				if (result.object.__typename === "User") {
					goto(`/${result.object.username}`);
					e.preventDefault();
					clearTimeout(timeout);
					closed = true;
				} else if (result.object.__typename === "Category") {
					goto(`/categories/${result.object.name.toLowerCase()}`);
					e.preventDefault();
					clearTimeout(timeout);
					closed = true;
				}
			}
		}
	}

	const client = getContextClient();

	let queryInput: HTMLInputElement;
	let query = "";

	let timeout: NodeJS.Timeout | number;
	let results: SearchResult[] | undefined;
	let closed = false;

	function search(query: string) {
		clearTimeout(timeout);
		timeout = setTimeout(() => {
			if (query) {
				searchQuery(client, query).then((res) => {
					closed = false;
					if (res.data) {
						results = res.data.resp.results as SearchResult[];
					}
				});
			} else {
				results = undefined;
			}
		}, 500);
	}

	$: search(query);

	$: resultsVisible = !closed && results?.length && results?.length > 0;
</script>

<search class:results-visible={resultsVisible}>
	<MouseTrap on:close={() => (closed = true)}>
		<form method="get" action="/search" on:submit={onSubmit}>
			<input
				name="q"
				type="text"
				placeholder="Search"
				autocomplete="off"
				bind:this={queryInput}
				bind:value={query}
				on:focus={() => (closed = false)}
				on:keydown|stopPropagation
			/>
			<button class="search-button" type="submit">
				<span class="sr-only">Search</span>
				<Fa icon={faMagnifyingGlass} size="1.2x" />
			</button>
		</form>
		{#if resultsVisible && results}
			<ul class="results">
				{#each results as result}
					<li>
						{#if result.object.__typename === "User"}
							<User user={result.object} on:close={() => (closed = true)} />
						{:else if result.object.__typename === "Category"}
							<Category category={result.object} on:close={() => (closed = true)} />
						{/if}
					</li>
				{/each}
			</ul>
		{/if}
	</MouseTrap>
</search>

<style lang="scss">
	@import "../../assets/styles/variables.scss";

	search {
		/* First, take 25rem and then shrink by a factor of 1 */
		flex: 0 1 25rem;

		position: relative;
	}

	form {
		display: flex;
		justify-content: center;
		align-items: stretch;

		background-color: $bgColor;
		border: 1px solid $borderColor;
		border-radius: 0.5rem;

		transition: border-color 0.25s;

		&:focus-within {
			border-color: $primaryColor;
			background-color: black;
		}

		input {
			flex-grow: 1;
			width: 6rem;
			height: 2.5rem;
			padding: 0.5rem 1rem;
			font: inherit;

			background: none;
			outline: none;
			border: none;

			color: $textColor;
			font-weight: 500;
			outline: none;

			&::placeholder {
				color: $textColorLight;
			}
		}

		.search-button {
			border-radius: 0 0.5rem 0.5rem 0;

			height: 2.5rem;
			padding: 0.75rem;
			color: $textColor;
			cursor: pointer;

			display: flex;
			align-items: center;
		}

		input:focus + .search-button {
			background-color: $bgColor;
		}
	}

	.results {
		position: absolute;
		top: calc(100% + 0.5rem);
		left: 0;
		right: 0;
		z-index: 1;

		margin: 0;
		padding: 0.5rem;
		list-style: none;

		background-color: $bgColor2;
		border: 1px solid $borderColor;
		border-radius: 0.5rem;
		transition: border-color 0.25s;

		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}
</style>
