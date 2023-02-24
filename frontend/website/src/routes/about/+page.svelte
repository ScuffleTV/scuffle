<svelte:head>
	<title>Scuffle - About</title>
</svelte:head>

<div class="body">
	<div class="inner">
		<div class="container header">
			<span class="blurred-circle blue pos-1" />
			<h1 class="wordmark">Scuffle.tv</h1>
			<p class="heading-description">
				We’re building a streaming platform.
				<br />
				A <span class="highlight">community driven streaming platform</span> that is free and open source.
			</p>
		</div>
		<div class="section">
			<span class="blurred-circle blue pos-2" />
			<h2>For Fun&trade;</h2>
			<p>
				Scuffle aims to be a teaching tool to help people learn about live streaming and software
				engineering. Our goals are to make a community driven platform where anyone can contribute
				and learn as well as provide insight onto how running a streaming platform works in the real
				world. We are completely open to suggestions and feedback, so feel free to reach out to us
				on our
				<a href="https://scuffle.tv/discord" target="_blank" rel="noopener noreferrer">Discord</a>
			</p>
		</div>
		<div class="section center">
			<h1>How we plan to do all this?</h1>
			<div class="bar-container">
				<div class="bar">
					<div class="rust-bar flex-text">
						<p>90% Rust</p>
						<p class="magic-text">10% Magic</p>
					</div>
					<div class="magic-bar" />
				</div>
			</div>
			<p>
				With the power of <span class="highlight">Rust</span> and a little
				<span class="highlight">Magic</span> anything is possible.
			</p>
		</div>
		<div class="section">
			<h2>What's Rust?</h2>
			<p>
				Rust is a low level systems programming language. Known for its speed, safety, and
				concurrency. Designed to build fast, reliable, and efficient software. We plan on using Rust
				for the backend and video infrastructure of the platform. On the frontend we plan on using
				SvelteKit with WebAssembly for the video player.
			</p>
		</div>
		<div class="section center">
			<span class="blurred-circle brown pos-3" />
			<h1>What makes it community driven?</h1>
			<p>
				The project is fully opensource. Made by a community of people who want to learn and build
				something together. We plan on having everything be as transparent and public, so that
				anyone can contribute and see how we are using your contributions. Our project is hosted on
				GitHub, and we have a Discord server where you can chat with us and other contributors.
			</p>
		</div>
		<div class="section center">
			<h1>Aren’t streaming platforms super expensive?</h1>
			<p>
				Yes! Streaming platforms are very expensive to run. A lot of streaming platforms are owned
				by large companies, and they have the resources to run these platforms. We are not. We are a
				small group of people who enjoy streaming and want to build a platform for others to enjoy.
				A key aspect in our design will be to make the platform run as cost effectively as possible
				whilst not sacrificing quality or features in the process. Which includes anyone being able
				to go live for free and not having to pay for a subscription to watch streams or use the
				platform.
			</p>
		</div>
		<div class="section">
			<span class="blurred-circle blue pos-4" />
			<span class="blurred-circle green pos-5" />
			<h2>How can I help?</h2>
			<p>
				You can start by joining our Discord, and throwing ideas our way on what we can do to make
				it better. If you are a developer, you can check out our GitHub and our Roadmap and see if
				there is any ticket that stands out to you!
			</p>
		</div>
		<div class="buttons-container center">
			<a href="https://github.com/scuffletv" target="_blank" rel="noopener noreferrer">
				<button class="github-button">GitHub</button>
			</a>
			<a href="https://scuffle.tv/discord" target="_blank" rel="noopener noreferrer">
				<button class="discord-button">Discord</button>
			</a>
		</div>
	</div>
</div>

<style lang="scss">
	h1,
	h2,
	a,
	p {
		z-index: 1;
		line-height: 1.35em;
	}

	.body {
		display: grid;
		padding: 0 1rem;
		place-items: center;
	}

	button {
		border: none;
		border-radius: 0.5rem;
		padding: 1rem 3rem;
		font-size: 1.2rem;
		font-weight: 600;
		cursor: pointer;
		transition: background 0.2s ease-in-out;
	}

	.discord-button {
		background: #5865f2;
		color: white;
	}

	.discord-button:hover {
		background: #7289da;
	}

	.github-button {
		background: #292929;
		color: white;
	}

	.github-button:hover {
		background: #3f3f3f;
	}

	.buttons-container {
		display: flex;
		flex-direction: row;
		justify-content: center;
		align-items: center;
		gap: 0.5rem;
		margin-top: 2rem;
	}

	.flex-text {
		display: flex;
		flex-direction: row;
		justify-content: space-between;
	}

	.flex-text > p {
		color: black;
	}

	.bar-container {
		display: flex;
		flex-direction: row;
		justify-content: center;
		align-items: center;
		min-width: min(60em, 80%);
		max-width: 50vw;
		margin: 0 auto;
	}

	.bar {
		display: flex;
		flex-direction: row;
		justify-content: center;
		align-items: center;
		width: 100%;
		margin-top: 1rem;
	}

	.magic-text {
		transform: translateX(2rem);
	}

	.rust-bar {
		width: 90%;
		height: 4rem;
		background: #dea584;
		border-radius: 0.5rem 0 0 0.5rem;
		text-align: left;
		padding-left: 1rem;
	}

	.magic-bar {
		width: 10%;
		height: 4rem;
		background: linear-gradient(to bottom right, #6fcbff, #fff, #ff72f9);
		border-radius: 0 0.5rem 0.5rem 0;
	}

	.section {
		position: relative;
		margin-top: 4rem;
		display: flex;
		flex-direction: column;
		text-align: center;
		justify-content: start;
	}

	.highlight {
		color: white;
		font-weight: bold;
	}

	.header {
		position: relative;
	}

	.heading-description {
		text-align: center;
		margin-top: -0.1rem;
	}

	p {
		color: rgb(163, 163, 163);
		font-weight: 400;
		font-size: 1.2rem;
	}

	.wordmark {
		font-family: "Comfortaa", sans-serif;
		font-weight: 700;
		font-size: min(6rem, 15vw);
		color: #ff7357;
		background: linear-gradient(#ff7357, #ff4c6d);
		background-clip: text;
		-webkit-linear-gradient: (#ff7357, #ff4c6d);
		-webkit-background-clip: text;
		-webkit-text-fill-color: transparent;
	}

	.blurred-circle {
		position: absolute;
		width: 30rem;
		height: 30rem;
		overflow: hidden;

		animation: scale-up-center 7s cubic-bezier(0.39, 0.575, 0.565, 1) infinite both;

		@keyframes scale-up-center {
			0% {
				transform: scale(1);
			}
			33% {
				transform: scale(1.1);
			}
			66% {
				transform: scale(0.9);
			}
			100% {
				transform: scale(1);
			}
		}

		&.blue {
			background: radial-gradient(
				50% 50% at 50% 50%,
				rgba(8, 97, 221, 0.3) 0%,
				rgba(0, 0, 0, 0) 100%
			);
		}
		&.brown {
			background: radial-gradient(
				50% 50% at 50% 50%,
				rgba(235, 138, 81, 0.259) 0%,
				rgba(0, 0, 0, 0) 100%
			);
		}
		&.green {
			background: radial-gradient(
				50% 50% at 50% 50%,
				rgba(136, 255, 142, 0.35) 0%,
				rgba(0, 0, 0, 0) 100%
			);
		}
		&.pos-1 {
			top: -2rem;
			left: 35rem;
		}
		&.pos-2 {
			top: 0rem;
			left: -10rem;
		}
		&.pos-3 {
			top: -15rem;
			left: 25rem;
		}
		&.pos-4 {
			top: 2rem;
			left: -20rem;
		}
		&.pos-5 {
			top: 5rem;
			left: 35rem;
		}
	}

	.container {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
	}

	.inner {
		margin: 5rem 0;
		display: flex;
		flex-direction: column;
		flex-grow: 1;
		max-width: min(60rem, 90vw);
	}
</style>
