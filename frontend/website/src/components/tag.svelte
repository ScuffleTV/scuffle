<script lang="ts">
	export let content: string;
</script>

<div class="tag">
	<span>{content}</span>
	<div class="tag-click" />
</div>

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
		transition: color 0.1s ease-in-out;

		cursor: default;
		user-select: none;
		position: relative;
		&:hover {
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
		&:active {
			.tag-click {
				/* Where do those colors come from? They're nowhere else */
				background-color: #2fc0f465;
				border-color: #52e2f2c4;
				transition: unset;
			}
		}
	}

	.tag-click {
		transition:
			background-color 0.5s ease-in-out,
			border 0.5s ease-in-out;
		position: absolute;
		top: 0;
		left: 0;
		width: 100%;
		height: 100%;
		background-color: transparent;
		border: 0.75px solid transparent;
	}
</style>
