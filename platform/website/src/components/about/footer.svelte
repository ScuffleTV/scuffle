<script lang="ts">
	import { PUBLIC_TWITTER_HANDLE } from "$env/static/public";
	import LogoText from "../icons/logo-text.svelte";
</script>

<footer>
	<div class="left">
		<div class="logo">
			<LogoText />
		</div>
		<div class="link-list">
			<h3>Socials</h3>
			<a href="https://discord.gg/scuffle" title="Join the cord!">Discord</a>
			<a href="https://github.com/ScuffleTV">GitHub</a>
			<a href="https://bytes.scuffle.tv/">Our Blog</a>
			<a href="https://x.com/{PUBLIC_TWITTER_HANDLE}">Twitter</a>
		</div>
		<div class="link-list">
			<h3>Help</h3>
			<a
				href="https://github.com/ScuffleTV/scuffle/issues/new?assignees=ScuffleTV%2Freviewers&labels=feature+request&projects=&template=feature-request.yml&title=%5BFeature%5D%3A+Scuffle+needs+%5B...%5D"
				>Suggest a feature</a
			>
			<a
				href="https://github.com/ScuffleTV/scuffle/issues/new?assignees=ScuffleTV%2Freviewers&labels=bug&projects=&template=bugreport.yml&title=%5BBug%5D%3A+Scuffle+has+a+bug+here+%5B...%5D"
				>Report a bug</a
			>
			<a href="https://github.com/ScuffleTV/scuffle/security/advisories/new"
				>Report a vulnerabilty</a
			>
		</div>
		<div class="link-list">
			<h3>Support us</h3>
			<a href="https://opencollective.com/scuffle">Open Collective</a>
		</div>
	</div>
	<span class="copyright">&copy; {new Date().getFullYear()} Scuffle</span>
	<img class="fishinge" src="/Fishinge.webp" width="40" alt="Fishinge" />
</footer>

<style lang="scss">
	@import "../../assets/styles/variables.scss";

	footer {
		background-color: $bgColor;
		position: relative;

		padding: 2rem 4rem;
		display: flex;
		justify-content: space-between;
		gap: 4rem;
		flex-wrap: wrap;

		.left {
			display: flex;
			gap: 4rem;
			flex-wrap: wrap;
		}

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
				margin-bottom: 0.25rem;
				font-size: 1rem;
				font-weight: 700;
			}

			a {
				color: $textColorLighter;
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
