<script lang="ts">
	import BlogPost from "$/components/about/blog-post.svelte";
	import Footer from "$/components/about/footer.svelte";
import LogoText from "$/components/icons/logo-text.svelte";
	import { PUBLIC_BLOG_API_KEY, PUBLIC_TWITTER_HANDLE } from "$env/static/public";
	import { faDiscord, faGithub } from "@fortawesome/free-brands-svg-icons";
	import { faCode, faHandHoldingDollar, faHeart, faPenNib, faRightToBracket, faVideo } from "@fortawesome/free-solid-svg-icons";
	import Fa from "svelte-fa";

	const BLOG_ENDPOINT = `https://bytes.scuffle.tv/ghost/api/content/posts/?key=${PUBLIC_BLOG_API_KEY}&include=authors&limit=2&fields=title,primary_author,url,excerpt,published_at`;

	async function fetchPosts() {
		const res = await fetch(BLOG_ENDPOINT);
		return await res.json();
	}
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
		<div class="hero-section">
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
				<span class="announcement">
					<span class="new">New</span>
					<span>Introducing Scuffle Beta</span>
				</span>
				<h1 class="big-text">
					<span class="bold">Low latency</span>
					<br>
					<span class="bold">community first</span>
					<br>
					<span>Live-streaming</span>
				</h1>
				<div class="buttons">
					<a href="/sign-up" class="button primary sign-up">
						<Fa icon={faRightToBracket} />
						Sign up
					</a>
					<a href="https://discord.gg/scuffle" class="button secondary">
						<Fa icon={faDiscord} />
						Join
					</a>
				</div>
			</div>
			<div class="image glow"></div>
		</div>
		<div class="features">
			<section class="community glow">
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
			<section class="blog">
				<div class="heading">
					<h2>Scuffle Engineering Blog</h2>
					<Fa icon={faPenNib} />
				</div>
				<div class="posts">
					{#await fetchPosts()}
						<p>Loading...</p>
					{:then posts}
					{#each posts.posts as post}
							<BlogPost
								title={post.title}
								excerpt={post.excerpt}
								author={post.primary_author}
								url={post.url}
								published_at={new Date(post.published_at)}
							/>
						{/each}
					{:catch error}
						<p>{error}</p>
					{/await}
				</div>
				<a class="read-more" href="https://bytes.scuffle.tv/">
					Read more
					<span class="arrow">-></span>
				</a>
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
					<a href="https://opencollective.com/scuffle" class="button secondary">
						<Fa icon={faHandHoldingDollar} />
						Donate
					</a>
				</div>
			</section>
		</div>
	</main>
	<Footer />
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

	.glow {
		--spread: 6rem;
		box-shadow: 0 0 8rem var(--spread) rgba($primaryColor, 0.1);
	}

	.logo {
		position: absolute;
		top: 2rem;
		left: 4rem;
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
		display: flex;
		flex-direction: column;
		align-items: center;
		padding: 0 4rem;
	}

	.announcement {
		display: flex;
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
			border-color: $textColor;
		}

		filter: drop-shadow(0 0 1rem rgba(255, 255, 255, 0.25));

		&:hover, &:focus-visible {
			filter: drop-shadow(0 0 2rem rgba(255, 255, 255, 0.25));
		}

		&.sign-up {
			position: relative;
			border: none;

			--border-width: 2px;

			&:hover{
				&::after {
					opacity: 1;
					transform: rotate(359deg);
				}
			}

			&::before {
				content: "";
				position: absolute;
				top: var(--border-width);
				left: var(--border-width);
				bottom: var(--border-width);
				right: var(--border-width);
				z-index: -1;
				border-radius: calc(0.75rem - var(--border-width));
				background-color: white;
			}

			overflow: hidden;

			&::after {
				content: "";
				position: absolute;
				top: -100%;
				left: -25%;
				bottom: -100%;
				right: -25%;
				z-index: -2;
				background: conic-gradient(
					hsl(0deg 100% 67%),
					hsl(40deg 100% 67%),
					hsl(80deg 100% 67%),
					hsl(120deg 100% 67%),
					hsl(160deg 100% 67%),
					hsl(200deg 100% 67%),
					hsl(240deg 100% 67%),
					hsl(280deg 100% 67%),
					hsl(320deg 100% 67%),
				);

				transition: opacity 0.2s, transform 0.5s;
				opacity: 0;
				transform: rotate(0deg);
			}
		}
	}

	.hero-section {
		width: 100%;
		max-width: 80rem;
		min-height: 100svh;
		min-height: 100vh;

		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 4rem;
		flex-wrap: wrap;
	}

	.text-cta {
		grid-column: 1 / 4;
		grid-row: 1;

		display: flex;
		flex-direction: column;
		align-items: flex-start;
		gap: 1rem;

		white-space: nowrap;

		& > .background-noise {
			position: absolute;
			top: 0;
			left: 0;
			bottom: 0;
			right: 0;
			max-width: 100vw;
			width: 100%;
			height: 100vh;
			height: 100svh;
			z-index: -1;

			-webkit-mask-image: linear-gradient(120deg, rgba(255, 255, 255, 0.15) 0%, transparent 80%);
			mask-image: linear-gradient(120deg, rgba(255, 255, 255, 0.15) 0%, transparent 80%);
			-webkit-mask-repeat: no-repeat;
  			mask-repeat: no-repeat;
		}

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
			gap: 1rem;

			margin-top: 1rem;
			font-size: 1.5rem;
		}
	}

	@property --border-angle {
		syntax: "<angle>";
		inherits: true;
		initial-value: 0deg;
	}

	.image {
		grid-column: 5 / 8;
		grid-row: 1;

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
			background-color: black;
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

			background: conic-gradient(
				transparent,
				$primaryColor,
			);

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

	.features {
		margin: 8rem 0;

		display: grid;
		grid-template-areas: "community emotes" "blog blog" "code code";
		gap: 8rem;

		max-width: 60rem;
	}

	@media screen and (max-width: $mobileBreakpoint) {
		.features {
			grid-template-areas: "community" "emotes" "code";
			gap: 4rem;
		}
	}

	section {
		border-radius: 1rem;

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
			line-height: 1.1em;
		}

		.buttons {
			margin-top: 2rem;
			display: flex;
			gap: 1rem;
		}

		&.community {
			--spread: 2rem;
			grid-area: community;

			padding: 2rem;
			color: black;
			background-color: $primaryColor;
		}

		&.emotes {
			grid-area: emotes;

			padding: 2rem;
			color: $textColor;

			.caption, h2 {
				color: $primaryColor;
			}
		}

		&.blog {
			grid-area: blog;

			color: $textColor;

			display: flex;
			flex-direction: column;
			gap: 1rem;

			.heading {
				display: flex;
				align-items: center;
				justify-content: space-between;
				gap: 1rem;

				margin-bottom: 1rem;
				font-size: 1.6rem;
				color: $primaryColor;

				h2 {
					font-size: 1.8rem;
					font-weight: 500;
					margin: 0;
					line-height: 1.1;
				}
			}

			.posts {
				display: flex;
				gap: 2rem;
			}

			.read-more {
				align-self: flex-end;
				color: $textColor;
				text-decoration: none;

				& > .arrow {
					display: inline-block;
					transform: translateX(0);
					transition: transform 0.2s;
				}

				&:hover, &:focus-visible {
					& > .arrow {
						transform: translateX(0.25rem);
					}
				}
			}
		}

		&.code {
			grid-area: code;

			padding: 2rem;
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
</style>
