<script lang="ts">
	import Fa from "svelte-fa";
	import { faMagnifyingGlass } from "@fortawesome/free-solid-svg-icons";
	import { getContextClient } from "@urql/svelte";
	import { graphql } from "$/gql";
	import MouseTrap from "../mouse-trap.svelte";
	import DefaultAvatar from "../user/default-avatar.svelte";
	import { viewersToString } from "$/lib/utils";
	import { goto } from "$app/navigation";

	function onSubmit(e: Event) {
		if (query) {
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
				if (result.__typename === "UserSearchResult") {
					goto(`/${result.user.username}`);
				} else if (result.__typename === "CategorySearchResult") {
					goto(`/categories/${result.category.name.toLowerCase()}`);
				}
			}
			e.preventDefault();
		}
	}

	const client = getContextClient();

	function searchQuery(query: string) {
		console.log("Searching for", query);
		return client
			.query(
				graphql(`
					query Search($query: String!) {
						results: search(query: $query) {
							users {
								similarity
								user {
									id
									username
									displayName
									displayColor {
										color
										hue
										isGray
									}
									channel {
										title
										liveViewerCount
										category {
											name
										}
									}
								}
							}
							categories {
								similarity
								category {
									name
								}
							}
						}
					}
				`),
				{ query },
				{ requestPolicy: "network-only" },
			)
			.toPromise();
	}

	type SearchResults = Exclude<
		Awaited<ReturnType<typeof searchQuery>>["data"],
		undefined
	>["results"];
	type UserResult = SearchResults["users"][number];
	type CategoryResult = SearchResults["categories"][number];

	let queryInput: HTMLInputElement;
	let query = "";

	let timeout: NodeJS.Timeout | number;
	let results: (UserResult | CategoryResult)[] | undefined;
	let closed = false;

	function search(query: string) {
		clearTimeout(timeout);
		timeout = setTimeout(() => {
			if (query) {
				searchQuery(query).then((res) => {
					closed = false;
					if (res.data) {
						results = [...res.data.results.users, ...res.data.results.categories];
						results.sort((a, b) => {
							if (a.similarity > b.similarity) return -1;
							if (a.similarity < b.similarity) return 1;
							return 0;
						});
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
				placeholder="SEARCH"
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
						{#if result.__typename === "UserSearchResult"}
							<a on:click={() => (closed = true)} href="/{result.user.username}">
								<div class="avatar">
									<DefaultAvatar
										userId={result.user.id}
										displayColor={result.user.displayColor}
										size={2.5 * 16}
									/>
								</div>
								<div class="text-container">
									<span class="name">
										<span class:offline={typeof result.user.channel.liveViewerCount !== "number"}
											>{result.user.displayName}</span
										>
										{#if typeof result.user.channel.liveViewerCount === "number" && result.user.channel.category?.name}
											<span class="category">• {result.user.channel.category.name}</span>
										{/if}
									</span>
									{#if typeof result.user.channel.liveViewerCount === "number" && result.user.channel.title}
										<span class="title">{result.user.channel.title}</span>
									{/if}
								</div>
								{#if typeof result.user.channel.liveViewerCount === "number"}
									<span class="live-viewers"
										>{viewersToString(result.user.channel.liveViewerCount)}</span
									>
								{/if}
							</a>
						{:else if result.__typename === "CategorySearchResult"}
							<a
								on:click={() => (closed = true)}
								href="/categories/{result.category.name.toLowerCase()}"
							>
								<div class="avatar">
									<img src="/categories/minecraft.png" alt="Category banner" />
								</div>
								<div class="text-container">
									<span class="name">{result.category.name}</span>
								</div>
							</a>
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

		a {
			color: $textColor;
			text-decoration: none;
			padding: 0.5rem;

			display: flex;
			gap: 0.5rem;
			align-items: center;

			border-radius: 0.25rem;
			background-color: $bgColor;

			transition: background-color 0.2s;

			&:hover,
			&:focus-visible {
				background-color: $bgColorLight;
			}

			.avatar {
				display: flex;
				justify-content: center;
				width: 2.5rem;
				height: 2.5rem;

				& > img {
					height: 100%;
				}
			}

			.text-container {
				flex-grow: 1;

				display: flex;
				flex-direction: column;
				justify-content: center;
				gap: 0.4rem;

				overflow: hidden;
			}

			.name,
			.title {
				overflow: hidden;
				text-overflow: ellipsis;
				white-space: nowrap;
			}

			.name {
				color: $textColor;
				font-size: 1rem;
				font-weight: 500;
			}

			.title,
			.category {
				color: $textColorLight;
				font-size: 0.865rem;
				font-weight: 500;
			}

			.live-viewers {
				font-size: 0.865rem;
				font-weight: 500;

				margin-right: 0.2rem;

				white-space: nowrap;

				&::before {
					content: "";
					display: inline-block;
					width: 0.4rem;
					height: 0.4rem;
					background-color: $liveColor;
					border-radius: 50%;
					margin-right: 0.4rem;
					margin-bottom: 0.1rem;
				}
			}

			.offline {
				color: $textColorLight;
			}
		}
	}
</style>
