<script context="module" lang="ts">
	export type Author = {
		name: string;
		profile_image: string;
	};
</script>

<script lang="ts">
	export let title: string;
	export let url: string;
	export let excerpt: string;
	export let author: Author;
	export let published_at: Date;
</script>

<article>
	<a href={url}>
		<h3>{title}</h3>
		<p>{excerpt}</p>
		<div class="author">
			<img src={author.profile_image} alt={author.name} />
			<span>{author.name}</span>
		</div>
		<span class="date">{published_at.toLocaleDateString("en-US", {
			year: "numeric",
			month: "short",
			day: "numeric",
		})}</span>
	</a>
</article>

<style lang="scss">
	@import "../../assets/styles/variables.scss";

	a {
		display: grid;
		grid-template-columns: repeat(2, auto);
		grid-template-rows: 1fr auto auto;
		gap: 1.5rem;
		height: 100%;
		position: relative;

		color: $textColor;
		text-decoration: none;
		padding: 2.5rem;
		border-radius: 1rem;
		background-color: $bgColor;

		border: 1px solid transparent;
		transition: border-color 0.2s;
		&:hover, &:focus-visible {
			border-color: $textColorLight;
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
			font-weight: 700;
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
