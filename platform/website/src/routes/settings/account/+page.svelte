<script>
	import SectionContainer from "$/components/settings/section-container.svelte";
	import Section from "$/components/settings/section.svelte";
	import StatusBar, { Status } from "$/components/settings/status-bar.svelte";
	import { graphql } from "$/gql";
	import { user } from "$/store/auth";
	import {
		faCheckCircle,
		faDownload,
		faPen,
		faShieldHalved,
	} from "@fortawesome/free-solid-svg-icons";
	import { getContextClient } from "@urql/svelte";
	import Fa from "svelte-fa";
	import { z } from "zod";

	//TODO: Improve detail texts

	const client = getContextClient();

	let email = $user?.email;
	$: emailValid = z.string().email().safeParse(email).success;
	$: emailChanged = email !== $user?.email;
	let emailRevealed = false;

	let status = Status.Unchanged;

	$: status = emailChanged ? Status.Changed : Status.Unchanged;

	function saveEmail() {
		if (email) {
			status = Status.Saving;
			const request = {
				query: graphql(`
					mutation SetEmail($email: String!) {
						user {
							resp: email(email: $email) {
								email
								emailVerified
							}
						}
					}
				`),
				variables: {
					email,
				},
			};

			client
				.mutation(request.query, request.variables, {
					requestPolicy: "network-only",
				})
				.toPromise()
				.then((result) => {
					if (result.data) {
						email = result.data.user.resp.email;
						if ($user) {
							$user.email = result.data.user.resp.email;
							$user.emailVerified = result.data.user.resp.emailVerified;
						}
					} else if (result.error && $user) {
						email = $user.email;
					}
				});
		}
	}

	function saveChanges() {
		if (emailChanged) {
			saveEmail();
		}
	}
</script>

{#if $user}
	<SectionContainer>
		<Section
			title="Email"
			details="Your email address is used to log in to your account."
			on:reset={() => (email = $user?.email)}
			showReset={emailChanged}
		>
			<div class="input-container">
				<input
					type="email"
					class="input email"
					class:revealed={emailRevealed}
					disabled={!emailRevealed}
					bind:value={email}
				/>
				{#if !emailRevealed}
					<button class="reveal-button" on:click={() => (emailRevealed = true)}>
						<Fa icon={faPen} />
					</button>
				{/if}
			</div>
			{#if !emailValid}
				<span class="error message">Invalid email address</span>
			{:else if $user.emailVerified}
				<span class="success message">Verified</span>
			{:else if $user.email === email}
				<span class="error message">Unverified</span>
			{/if}
		</Section>
		<Section title="Password" details="Your password is used to log in to your account.">
			<button class="button primary change-password">
				<Fa icon={faPen} />
				Change Password
			</button>
		</Section>
		<Section title="2-Factor-Authentication" details="2FA adds more security to your account.">
			<div class="input big">
				<div class="twofa-state">
					<Fa icon={faShieldHalved} />
					<span>Enabled</span>
				</div>
				<div class="buttons">
					<button class="button primary">
						<Fa icon={faCheckCircle} />
						Enable 2FA
					</button>
					<button class="button secondary">
						<Fa icon={faDownload} />
						Download Backup Codes
					</button>
				</div>
			</div>
		</Section>
		<StatusBar {status} on:save={saveChanges} saveDisabled={!emailValid} />
	</SectionContainer>
{/if}

<style lang="scss">
	@import "../../../assets/styles/settings.scss";

	.input.email:not(.revealed) {
		color: transparent;
		pointer-events: none;
		user-select: none;
		text-shadow: 5px 0 10px $textColor;
	}

	.input-container {
		margin-top: 0.5rem;
		position: relative;

		& > .input {
			margin-top: 0;
			width: 100%;
		}
	}

	.reveal-button {
		position: absolute;
		top: 0;
		right: 0.5rem;
		bottom: 0;

		color: $textColorLight;
	}

	.change-password {
		margin-top: 0.5rem;
	}

	.twofa-state {
		font-size: 2rem;

		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 0.5rem;

		span {
			font-size: 1rem;
			font-weight: 500;
		}
	}
</style>
