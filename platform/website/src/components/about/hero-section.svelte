<script lang="ts">
	import { faDiscord } from "@fortawesome/free-brands-svg-icons";
	import { faRightToBracket } from "@fortawesome/free-solid-svg-icons";
	import Fa from "svelte-fa";
</script>

<div class="hero-section">
	<svg class="background-noise" xmlns="http://www.w3.org/2000/svg">
		<filter id="noiseFilter">
			<feTurbulence type="fractalNoise" baseFrequency="0.5" numOctaves="3" stitchTiles="stitch" />
		</filter>

		<rect width="100vw" height="100vh" filter="url(#noiseFilter)" />
	</svg>
	<div class="text-cta">
		<span class="announcement">
			<span class="new">New</span>
			<span>Introducing Scuffle Beta</span>
		</span>
		<h1 class="big-text">
			<span class="bold">Low latency</span>
			<br />
			<span class="bold">community first</span>
			<br />
			<span>Live-streaming</span>
		</h1>
		<div class="buttons">
			<a href="/sign-up" class="button primary rainbow">
				<Fa icon={faRightToBracket} />
				Sign up
			</a>
			<a href="https://discord.gg/scuffle" class="button secondary rainbow">
				<Fa icon={faDiscord} />
				Join
			</a>
		</div>
	</div>
	<div class="image"></div>
</div>

<style lang="scss">
	@import "../../assets/styles/variables.scss";

	.announcement {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 0.5rem;
		font-size: 1.1rem;
		color: $primaryColor;

		.new {
			font-size: 1rem;
			font-weight: 700;

			background-color: $primaryColor;
			color: white;
			padding: 0.25rem 0.5rem;
			border-radius: 0.5rem;
		}
	}

	.button {
		padding: 0.75rem 1rem;
		font-size: 1.25rem;
		font-weight: 500;
		border-radius: 0.75rem;

		display: flex;
		align-items: center;
		gap: 0.5rem;

		&.secondary {
			color: $textColor;
		}

		filter: drop-shadow(0 0 1rem rgba(255, 255, 255, 0.25));

		&:hover,
		&:focus-visible {
			filter: drop-shadow(0 0 2rem rgba(255, 255, 255, 0.25));
		}
	}

	.hero-section {
		width: 100%;
		min-height: 100vh;
		min-height: 100svh;

		padding: 5rem;

		display: flex;
		align-items: center;
		gap: 4rem;
		flex-wrap: wrap;
		justify-content: space-evenly;

		position: relative;

		.background-noise {
			position: absolute;
			top: 0;
			left: 0;
			width: 100%;
			height: 100%;
			z-index: -2;

			-webkit-mask-image: linear-gradient(120deg, rgba(255, 255, 255, 0.15) 0%, transparent 80%);
			mask-image: linear-gradient(120deg, rgba(255, 255, 255, 0.15) 0%, transparent 80%);
			-webkit-mask-repeat: no-repeat;
			mask-repeat: no-repeat;
		}
	}

	.text-cta {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		gap: 1rem;

		& > .big-text {
			font-size: 4.5rem;
			font-weight: 300;
			color: $textColor;
			line-height: 1.1;

			& > .bold {
				font-weight: 800;
			}
		}

		.buttons {
			display: flex;
			flex-wrap: wrap;
			gap: 1rem;

			margin-top: 1rem;
			font-size: 1.5rem;
		}
	}

	.image {
		box-shadow: 0 0 8rem 6rem rgba($primaryColor, 0.1);
		width: 100%;
		max-width: 30rem;
		aspect-ratio: 1 / 1;
		border-radius: 1rem;
		position: relative;

		// When you want to see how this magic works, remove the background-color from the ::before pseudo-element

		// This covers the whole element except for a border of 1px on each side
		&::before {
			content: "";
			position: absolute;
			top: 1px;
			left: 1px;
			bottom: 1px;
			right: 1px;
			z-index: 1;
			border-radius: 1rem;
			background-color: $bgColor2;
		}

		overflow: hidden;

		// This lies behind the cover and is used to create the border
		&::after {
			content: "";
			position: absolute;
			top: -25%;
			left: -25%;
			bottom: -25%;
			right: -25%;
			z-index: -1;

			background: conic-gradient(transparent, $primaryColor);

			@keyframes spin {
				from {
					transform: rotate(0deg);
				}
				to {
					transform: rotate(360deg);
				}
			}

			animation: spin 10s linear infinite;
		}
	}

	@media screen and (max-width: $mobileBreakpoint) {
		.hero-section {
			padding: 2rem;
			padding-top: 6rem;
		}

		.text-cta {
			& > .big-text {
				font-size: 3rem;
			}

			.buttons {
				font-size: 1.25rem;
			}
		}
	}
</style>
