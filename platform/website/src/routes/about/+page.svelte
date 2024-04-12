<script lang="ts">
	import BlogPost from "$/components/about/blog-post.svelte";
	import Footer from "$/components/about/footer.svelte";
	import HeroSection from "$/components/about/hero-section.svelte";
	import LogoText from "$/components/icons/logo-text.svelte";
	import { PUBLIC_BLOG_API_KEY } from "$env/static/public";
	import { faGithub } from "@fortawesome/free-brands-svg-icons";
	import {
		faCircleExclamation,
		faCode,
		faHandHoldingDollar,
		faHeart,
		faPenNib,
		faVideo,
	} from "@fortawesome/free-solid-svg-icons";
	import Fa from "svelte-fa";

	// https://ghost.org/docs/content-api
	const BLOG_ENDPOINT = `https://bytes.scuffle.tv/ghost/api/content/posts/?key=${PUBLIC_BLOG_API_KEY}&include=authors&limit=2&fields=title,primary_author,url,excerpt,published_at`;

	async function fetchPosts() {
		const res = await fetch(BLOG_ENDPOINT, {
			headers: {
				Accept: "application/json",
			},
		});
		return await res.json();
	}
</script>

<svelte:head>
	<title>Scuffle - About</title>
</svelte:head>

<div class="content">
	<header>
		<a class="logo" href="/">
			<LogoText />
		</a>
	</header>
	<main>
		<HeroSection />
		<div class="features">
			<section class="emotes">
				<div class="caption">
					<span>EMOTES</span>
					<Fa icon={faHeart} />
				</div>
				<h2>Emotes for everyone</h2>
				<span>
					Effortless emotes for all users out of the box. No need to setup any complicated
					third-party apps.
				</span>
			</section>
			<section class="community">
				<div class="caption">
					<span>COMMUNITY FIRST</span>
					<Fa icon={faVideo} />
				</div>
				<h2>By viewers, for viewers</h2>
				<span>
					Built by the community, for the community. We're always listening to feedback and
					suggestions.
				</span>
			</section>
			<section class="blog">
				<div class="heading">
					<h2>Scuffle Engineering Blog</h2>
					<Fa icon={faPenNib} />
				</div>
				<div class="posts">
					{#await fetchPosts()}
						<BlogPost />
						<BlogPost />
					{:then posts}
						{#each posts.posts as post}
							<BlogPost data={post} />
						{/each}
					{:catch error}
						<div class="error">
							<Fa icon={faCircleExclamation} />
							Failed to fetch posts: {error}
						</div>
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
				<span>Scuffle is open source. Anyone is welcome to contribute to the project.</span>
				<div class="buttons">
					<a href="https://github.com/ScuffleTV" class="button primary rainbow">
						<Fa icon={faGithub} />
						GitHub
					</a>
					<a href="https://opencollective.com/scuffle" class="button secondary rainbow">
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

	.logo {
		position: absolute;
		z-index: 1;
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

		&:hover,
		&:focus-visible {
			filter: drop-shadow(0 0 2rem rgba(255, 255, 255, 0.25));
		}
	}

	.features {
		margin: 8rem 4rem;

		display: grid;
		grid-template-areas: "emotes community" "blog blog" "code code";
		grid-template-columns: 1fr 1fr;
		gap: 8rem 2rem;

		max-width: 60rem;
	}

	@media screen and (max-width: $mobileBreakpoint) {
		.features {
			grid-template-areas: "emotes" "community" "blog" "code";
			gap: 4rem;
			margin: 4rem 2rem;
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
			overflow-wrap: anywhere;
		}

		.buttons {
			margin-top: 2rem;
			display: flex;
			flex-wrap: wrap;
			gap: 1rem;
		}

		&.emotes {
			grid-area: emotes;

			box-shadow: 0 0 8rem 2rem rgba($primaryColor, 0.1);
			padding: 2rem;
			color: black;
			background-color: $primaryColor;
		}

		&.community {
			grid-area: community;

			padding: 2rem;
			color: $textColor;
			border: 2px solid $borderColor;

			.caption,
			h2 {
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
				display: grid;
				gap: 2rem;
				grid-template-columns: repeat(auto-fit, minmax(18rem, 1fr));
			}

			.error {
				display: flex;
				align-items: center;
				gap: 0.5rem;
				color: $textColorLight;
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

				&:hover,
				&:focus-visible {
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
				width: 75%;
				height: 100%;
				z-index: -1;

				background-image: url("/about/code.webp");
				background-repeat: no-repeat;

				-webkit-mask-image: linear-gradient(30deg, transparent 40%, rgba(0, 0, 0, 1) 100%);
				mask-image: linear-gradient(30deg, transparent 40%, rgba(0, 0, 0, 1) 100%);
				-webkit-mask-repeat: no-repeat;
				mask-repeat: no-repeat;
			}

			.caption,
			h2 {
				color: $primaryColor;
			}
		}
	}
</style>
