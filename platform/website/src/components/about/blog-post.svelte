<script context="module" lang="ts">
	export type Author = {
		name: string;
		profile_image: string;
	};

	export type Post = {
		title: string;
		url: string;
		excerpt: string;
		primary_author: Author;
		published_at: string;
	};
</script>

<script lang="ts">
	import Spinner from "../spinner.svelte";

	export let data: Post | undefined = undefined;

	$: publishedAtFormatted = data
		? new Date(data.published_at).toLocaleDateString("en-US", {
				year: "numeric",
				month: "short",
				day: "numeric",
		  })
		: undefined;
</script>

<article>
	{#if data}
		<a href={data.url} class="post">
			<h3>{data.title}</h3>
			<p>{data.excerpt}</p>
			<div class="author">
				<img src={data.primary_author.profile_image} alt={data.primary_author.name} />
				<span>{data.primary_author.name}</span>
			</div>
			<span class="date">{publishedAtFormatted}</span>
		</a>
	{:else}
		<div class="post loading">
			<Spinner />
		</div>
	{/if}
</article>

<style lang="scss">
	@import "../../assets/styles/variables.scss";

	.post {
		display: grid;
		grid-template-columns: repeat(2, auto);
		grid-template-rows: 1fr auto auto;
		gap: 1.5rem;
		height: 100%;
		position: relative;

		&.loading {
			grid-template-columns: auto;
			grid-template-rows: auto;
			place-items: center;
			height: 18rem;
		}

		color: $textColor;
		text-decoration: none;
		padding: 2.5rem;
		border-radius: 1rem;
		background-color: $bgColor;

		&:not(.loading) {
			border: 1px solid transparent;
			transition: border-color 0.2s;
			&:hover,
			&:focus-visible {
				border-color: $textColorLight;
			}
		}

		&::before {
			content: "";
			position: absolute;
			top: 0;
			left: 0;
			bottom: 0;
			right: 0;
			z-index: -2;
			border-radius: 1rem;
			box-shadow: 0 0 8rem 4rem rgba($primaryColor, 0.1);
		}

		h3 {
			grid-column: 1 / -1;

			margin: 0;
			color: $primaryColor;
			font-size: 1.8rem;
			font-weight: 600;
			line-height: 1.1em;
		}

		p {
			--max-lines: 3;

			grid-column: 1 / -1;

			margin: 0;
			overflow: hidden;
			text-overflow: ellipsis;
			line-height: 1.2em;
			max-height: calc(1.2rem * var(--max-lines));

			display: -webkit-box;
			-webkit-line-clamp: var(--max-lines);
			line-clamp: var(--max-lines);
			-webkit-box-orient: vertical;
		}

		.author {
			display: flex;
			align-items: center;
			gap: 0.5rem;

			img {
				width: 1.5rem;
				height: 1.5rem;
				border-radius: 50%;
			}
		}

		.date {
			justify-self: end;
			text-align: end;
			color: $textColorLight;
		}
	}
</style>
