<script lang="ts">
	import SectionContainer from "$/components/settings/section-container.svelte";
	import Section from "$/components/settings/section.svelte";
	import StatusBar, { Status } from "$/components/settings/status-bar.svelte";
	import { graphql } from "$/gql";
	import { user } from "$/store/auth";
	import { faCheckCircle, faPen, faXmarkCircle } from "@fortawesome/free-solid-svg-icons";
	import ShieldCheck from "$components/icons/settings/shield-check.svelte";
	import ShieldX from "$components/icons/settings/shield-x.svelte";
	import { getContextClient } from "@urql/svelte";
	import Fa from "svelte-fa";
	import { z } from "zod";
	import Enable2fa from "$/components/settings/account/enable-2fa.svelte";
	import Disable2fa from "$/components/settings/account/disable-2fa.svelte";
	import ChangePassword from "$/components/settings/account/change-password.svelte";
	import Field from "$/components/form/field.svelte";

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

	enum Dialog {
		None,
		ChangePassword,
		Enable2Fa,
		Disable2Fa,
	}

	let showDialog = Dialog.None;

	function closeDialog() {
		showDialog = Dialog.None;
	}
</script>

{#if $user}
	<SectionContainer>
		<Section
			title="Email"
			details="Primary contact for account management and notifications."
			on:reset={() => (email = $user?.email)}
			showReset={emailChanged}
		>
			<div class="input-container" class:revealed={emailRevealed}>
				<Field
					type="email"
					autocomplete="email"
					placeholder="Email"
					disabled={!emailRevealed}
					bind:value={email}
				>
					{#if !emailRevealed}
						<button class="reveal-button" on:click={() => (emailRevealed = true)}>
							<Fa icon={faPen} />
						</button>
					{/if}
				</Field>
			</div>
			{#if !emailValid}
				<span class="error message">Invalid email address</span>
			{:else if $user.emailVerified}
				<span class="success message">Verified</span>
			{:else if $user.email === email}
				<span class="error message">Unverified</span>
			{/if}
		</Section>
		<Section title="Password" details="Protect your account with a strong, unique password.">
			<button
				class="button primary change-password"
				on:click={() => (showDialog = Dialog.ChangePassword)}
			>
				<Fa icon={faPen} />
				Change Password
			</button>
		</Section>
		<Section
			title="2-Factor-Authentication"
			details="Add an extra layer of security to your account."
		>
			<div class="input big">
				<div class="twofa-state" class:enabled={$user.totpEnabled}>
					{#if $user.totpEnabled}
						<ShieldCheck />
						<span>Enabled</span>
					{:else}
						<ShieldX size={25} />
						<span>Disabled</span>
					{/if}
				</div>
				<div class="buttons">
					{#if $user.totpEnabled}
						<button class="button primary" on:click={() => (showDialog = Dialog.Disable2Fa)}>
							<Fa icon={faXmarkCircle} />
							Disable 2FA
						</button>
					{:else}
						<button class="button primary" on:click={() => (showDialog = Dialog.Enable2Fa)}>
							<Fa icon={faCheckCircle} />
							Enable 2FA
						</button>
					{/if}
				</div>
			</div>
		</Section>
		<StatusBar {status} on:save={saveChanges} saveDisabled={!emailValid} />
	</SectionContainer>
	{#if showDialog === Dialog.ChangePassword}
		<ChangePassword on:close={closeDialog} />
	{:else if showDialog === Dialog.Enable2Fa}
		<Enable2fa on:close={closeDialog} />
	{:else if showDialog === Dialog.Disable2Fa}
		<Disable2fa on:close={closeDialog} />
	{/if}
{/if}

<style lang="scss">
	@import "../../../../assets/styles/settings.scss";

	.message {
		font-size: 0.9rem;
		font-weight: 500;

		&.error {
			color: $errorColor;
		}

		&.success {
			color: $successColor;
		}
	}

	.input-container {
		margin-top: 0.5rem;

		&:not(.revealed) {
			:global(input) {
				color: transparent;
				pointer-events: none;
				user-select: none;
				text-shadow: 5px 0 10px $textColor;
			}
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

		filter: drop-shadow(0 0 20px rgba($errorColor, 0.5));

		&.enabled {
			filter: drop-shadow(0 0 20px rgba($successColor, 0.5));
		}

		span {
			font-size: 1rem;
			font-weight: 500;
		}
	}
</style>
