<script lang="ts">
	export let content: string;
</script>

<a class="tag" href="/categories/{content}">
	<span class="sr-only">Tag</span>
	{content}
</a>

<style lang="scss">
	@import "../assets/styles/variables.scss";

	$borderColor: #383838;
	$dashLength: 0.3rem;
	$spaceLength: 0.15rem;

	// https://codepen.io/amit_sheen/pen/xxZeyjO for generating the background-image

	.tag {
		padding: 0.25rem 0.5rem;
		font-size: 0.85rem;
		height: 1.5rem;
		font-weight: 500;
		background-image: repeating-linear-gradient(
				0deg,
				$borderColor,
				$borderColor $dashLength,
				transparent $dashLength,
				transparent $dashLength + $spaceLength,
				$borderColor $dashLength + $spaceLength
			),
			repeating-linear-gradient(
				90deg,
				$borderColor,
				$borderColor $dashLength,
				transparent $dashLength,
				transparent $dashLength + $spaceLength,
				$borderColor $dashLength + $spaceLength
			),
			repeating-linear-gradient(
				180deg,
				$borderColor,
				$borderColor $dashLength,
				transparent $dashLength,
				transparent $dashLength + $spaceLength,
				$borderColor $dashLength + $spaceLength
			),
			repeating-linear-gradient(
				270deg,
				$borderColor,
				$borderColor $dashLength,
				transparent $dashLength,
				transparent $dashLength + $spaceLength,
				$borderColor $dashLength + $spaceLength
			);
		background-size:
			1px 100%,
			100% 1px,
			1px 100%,
			100% 1px;
		background-position:
			0 0,
			0 0,
			100% 0,
			0 100%;
		background-repeat: no-repeat;
		color: $textColorLight;
		text-decoration: none;
		transition: color 0.1s ease-in-out;

		cursor: default;
		user-select: none;
		position: relative;

		&:hover,
		&:focus-visible {
			color: $textColor;
			$backgroundSize: 10rem;
			background-image: repeating-linear-gradient(
					0deg,
					$borderColor,
					$borderColor $backgroundSize,
					transparent
				),
				repeating-linear-gradient(90deg, $borderColor, $borderColor $backgroundSize, transparent),
				repeating-linear-gradient(180deg, $borderColor, $borderColor $backgroundSize, transparent),
				repeating-linear-gradient(270deg, $borderColor, $borderColor $backgroundSize, transparent);
		}
	}
</style>
