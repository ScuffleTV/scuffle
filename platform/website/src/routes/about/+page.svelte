<script lang="ts">
	import LogoText from "$/components/icons/logo-text.svelte";
	import { faGithub } from "@fortawesome/free-brands-svg-icons";
	import { faCode, faHandHoldingDollar, faHeart, faRightToBracket, faVideo } from "@fortawesome/free-solid-svg-icons";
	import Fa from "svelte-fa";
</script>

<svelte:head>
	<title>Scuffle</title>
</svelte:head>

<div class="content">
	<header>
		<a class="logo" href="/">
			<LogoText />
		</a>
	</header>
	<main>
		<div class="text-cta">
			<!-- Background Noise -->
			<svg class="background-noise" xmlns='http://www.w3.org/2000/svg'>
				<filter id='noiseFilter'>
					<feTurbulence
						type='fractalNoise'
						baseFrequency='0.5'
						numOctaves='3'
						stitchTiles='stitch'/>
				</filter>

				<rect width='100vw' height='100vh' filter='url(#noiseFilter)'/>
			</svg>
			<span class="big-text">
				<span class="bold">Low latency</span>
				<br>
				<span class="bold">community first</span>
				<br>
				<span>Live-streaming</span>
			</span>
			<a href="/sign-up" class="button primary">
				<Fa icon={faRightToBracket} />
				Sign up
			</a>
		</div>
		<div class="image"></div>
		<section class="community">
			<div class="caption">
				<span>COMMUNITY FIRST</span>
				<Fa icon={faVideo} />
			</div>
			<h2>By viewers, for viewers</h2>
			<span>Made by a community of people who actually care about their work.</span>
		</section>
		<section class="emotes">
			<div class="caption">
				<span>EMOTES</span>
				<Fa icon={faHeart} />
			</div>
			<h2>Emotes for everyone</h2>
			<span>Effortless emotes for all users out of the box. No need to setup any complicated third-party apps.</span>
		</section>
		<section class="code">
			<div class="caption">
				<span>OPEN SOURCE</span>
				<Fa icon={faCode} />
			</div>
			<h2>Contributions welcome</h2>
			<span>Scuffle is open source, meaning anyone can contribute to the project.</span>
			<div class="buttons">
				<a href="https://github.com/ScuffleTV" class="button primary">
					<Fa icon={faGithub} />
					GitHub
				</a>
				<a href="https://github.com/ScuffleTV" class="button secondary">
					<Fa icon={faHandHoldingDollar} />
					Donate
				</a>
			</div>
		</section>
	</main>
	<footer>
		<div class="logo">
			<LogoText />
		</div>
		<div class="link-list">
			<h3>Socials</h3>
			<a href="https://discord.gg/scuffle" title="Join the cord!">Discord</a>
			<a href="https://github.com/ScuffleTV">GitHub</a>
			<a href="https://bytes.scuffle.tv/">Our Blog</a>
			<a href="https://x.com/scuffle_tv">Twitter</a>
		</div>
		<div class="link-list">
			<h3>Support us</h3>
			<a href="https://opencollective.com/scuffle">Open Collective</a>
		</div>
		<span class="copyright">&copy; {new Date().getFullYear()} Scuffle</span>
		<img class="fishinge" src="/Fishinge.webp" width="40" alt="Fishinge" />
	</footer>
</div>

<style lang="scss">
	@import "../../assets/styles/variables.scss";

	:global(body) {
		background-color: $bgColor2;
	}

	:global(.page-body) {
		overflow: auto;
		position: relative;
	}

	.logo {
		position: absolute;
		top: 1.5rem;
		left: 2rem;
		font-size: 2rem;

		transition: filter 0.2s;

		&:hover {
			filter: drop-shadow(0 0 5rem $primaryColor);
		}
	}

	.content {
		grid-area: content;
	}

	main {
		display: grid;
		grid-template-columns: repeat(5, 1fr);
		grid-template-rows: 100vh;
		grid-template-rows: 100svh;
		gap: 8rem 5rem;

		margin: 0 auto;
		margin-bottom: 8rem;
		max-width: 80rem;
		width: 100%;
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
			border-color: $textColor;
		}

		&:hover {
			filter: drop-shadow(0 0 1rem rgba(255, 255, 255, 0.25));
		}
	}

	.text-cta {
		align-self: center;
		grid-column: 1 / 4;
		grid-row: 1;

		display: flex;
		flex-direction: column;
		gap: 2rem;
		align-items: flex-start;

		& > .background-noise {
			position: absolute;
			top: 0;
			left: 0;
			bottom: 0;
			right: 0;
			width: 100vw;
			height: 100vh;
			z-index: -1;

			-webkit-mask-image: linear-gradient(120deg, rgba(255, 255, 255, 0.15) 0%, transparent 80%);
			mask-image: linear-gradient(120deg, rgba(255, 255, 255, 0.15) 0%, transparent 80%);
			-webkit-mask-repeat: no-repeat;
  			mask-repeat: no-repeat;
		}

		& > .big-text {
			font-size: 4.5rem;
			font-weight: 400;
			color: $textColor;

			& > .bold {
				font-weight: 700;
			}
		}

		.button {
			font-size: 1.5rem;
		}
	}

	.image {
		justify-self: end;
		align-self: center;
		grid-column: 4 / 6;
		grid-row: 1;

		height: 30rem;
		width: 30rem;
		background-color: black;
		border-radius: 1rem;
		border: 1px solid $primaryColor;

		box-shadow: 0 0 8rem 2rem rgba($primaryColor, 0.1);
	}

	section {
		border-radius: 1rem;

		padding: 2rem;
		transition: margin 0.2s, padding 0.2s;
		&:hover {
			margin: -0.25rem;
			padding: 2.25rem;
		}

		.caption {
			display: flex;
			align-items: center;
			justify-content: space-between;
			// Set the icon size
			font-size: 1.5rem;

			span {
				// Reset caption font size
				font-size: 1rem;
				letter-spacing: 0.1rem;
			}
		}

		h2 {
			font-size: 2.5rem;
			font-weight: 700;
			margin: 1rem 0;
		}

		.buttons {
			margin-top: 2rem;
			display: flex;
			gap: 1rem;
		}

		&.community {
			grid-column: 1 / 3;

			color: black;
			background-color: $primaryColor;
		}

		&.emotes {
			grid-column: 4 / 6;

			color: $textColor;

			.caption, h2 {
				color: $primaryColor;
			}
		}

		&.code {
			grid-column: 1 / 6;

			border: 2px solid $primaryColor;
			color: $textColor;

			position: relative;

			&::before {
				content: "";
				position: absolute;
				top: 0;
				right: 0;
				bottom: 0;
				left: 0;
				z-index: -1;

				background-image: url("scuffle_code.png");
				background-size: 50%;
				background-repeat: no-repeat;
				background-position: right;

				-webkit-mask-image: linear-gradient(30deg, transparent 40%, rgba(0, 0, 0, 1) 100%);
				mask-image: linear-gradient(30deg, transparent 40%, rgba(0, 0, 0, 1) 100%);
				-webkit-mask-repeat: no-repeat;
  				mask-repeat: no-repeat;
			}

			.caption, h2 {
				color: $primaryColor;
			}
		}
	}

	footer {
		background-color: $bgColor;
		position: relative;

		padding: 2rem 4rem;
		display: grid;
		grid-template-columns: auto auto auto 1fr;
		grid-template-rows: auto;
		gap: 4rem;

		.logo {
			display: contents;
			font-size: 2rem;
		}

		.link-list {
			display: flex;
			flex-direction: column;
			gap: 0.25rem;

			h3 {
				margin: 0;
				margin-bottom: 0.5rem;
				font-size: 1rem;
				font-weight: 700;
			}

			a {
				color: $textColor;
				text-decoration: none;

				&:hover {
					text-decoration: underline;
				}
			}
		}

		.copyright {
			place-self: end end;

			font-size: 0.9rem;
			color: $textColorLight;
		}

		.fishinge {
			position: absolute;
			bottom: 0;
			right: 0;
			overflow: hidden;

			filter: saturate(0) opacity(0.25);
		}
	}
</style>
