<script lang="ts">
	import { faDiscord } from "@fortawesome/free-brands-svg-icons";
	import { faRightToBracket } from "@fortawesome/free-solid-svg-icons";
	import Fa from "svelte-fa";
</script>

<svelte:head>
	<title>Scuffle</title>
</svelte:head>

<div class="hero-section" role="none">
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
		justify-content: center;

		position: relative;
		background-color: rgba(255, 255, 255, 0.01);

		&::before {
			content: "";
			position: absolute;
			top: 0;
			left: 0;
			bottom: 0;
			right: 0;

			background: image-set(
				url("/about/1x.avif") type("image/avif"),
				url("/about/1x.webp") type("image/webp"),
				url("/about/1x.jpg") type("image/jpg"),
			);
			background-repeat: no-repeat;
			background-size: cover;
			background-position: center;

			opacity: 0.5;
			z-index: -1;
		}
	}

	.text-cta {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		gap: 1rem;
		max-width: 80rem;
		width: 100%;

		& > .big-text {
			font-size: 3rem;
			font-weight: 300;
			color: $textColor;
			line-height: 1.1;

			& > .bold {
				font-weight: 800;
				font-size: 1.5em;
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

	@media screen and (max-width: $mobileBreakpoint) {
		.hero-section {
			padding: 2rem;
			padding-top: 6rem;
		}

		.text-cta {
			& > .big-text {
				font-size: 2.25rem;
			}

			.buttons {
				font-size: 1.25rem;
			}
		}
	}
</style>
