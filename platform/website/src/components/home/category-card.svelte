<script lang="ts">
	import { viewersToString } from "$/lib/utils";
	import Fa from "svelte-fa";
	import { faUser } from "@fortawesome/free-regular-svg-icons";

	export let title: string;
	export let image: string;
	export let viewers: number;
</script>

<a
	class="category"
	href="/categories/{title.toLowerCase()}"
	aria-label="Category {title} with {viewersToString(viewers, true)}"
>
	<img src={image} alt="{title} thumbnail" />
	<span>{title}</span>
	<span class="sr-only">{viewersToString(viewers, true)}</span>
	<div class="info-container">
		<span class="title">{title}</span>
		<div class="viewers">
			<Fa icon={faUser} />
			<span>{viewersToString(viewers, true)}</span>
		</div>
	</div>
</a>

<style lang="scss">
	@import "../../assets/styles/variables.scss";

	.category {
		font-family: $sansFont;

		display: flex;
		flex-direction: column;
		gap: 0.25rem;
		cursor: pointer;
		text-decoration: none;

		position: relative;

		&:hover,
		&:focus-visible {
			& > img {
				transform: scale(1.25);
				filter: drop-shadow(0 0 0.5rem black);
			}

			& > .info-container {
				opacity: 1;
				transition: opacity 0.25s 0.05s;
			}
		}

		& > img {
			aspect-ratio: 3/4;
			transition: transform 0.25s;
		}

		& > span {
			color: $textColorLight;
			font-size: 0.95rem;
			font-weight: 500;
			white-space: nowrap;
			overflow: hidden;
			text-overflow: ellipsis;
		}
	}

	.info-container {
		position: absolute;
		width: 100%;
		aspect-ratio: 3/4;
		transform: scale(1.25);
		opacity: 0;
		background: linear-gradient(transparent 50%, rgba(0, 0, 0, 0.8) 80%, black 100%);
		transition: opacity 0s;

		display: flex;
		flex-direction: column;
		justify-content: flex-end;
		padding: 0.5rem;
		gap: 0.25rem;

		& > .title {
			color: $textColor;
			font-size: 1rem;
			font-weight: 500;
			line-height: 0.95rem;
		}

		& > .viewers {
			color: $liveColor;
			font-size: 0.8rem;
			font-weight: 500;

			display: flex;
			align-items: center;
			gap: 0.25rem;
		}
	}
</style>
