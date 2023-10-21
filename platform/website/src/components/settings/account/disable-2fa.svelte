<script lang="ts">
	import { createEventDispatcher } from "svelte";
	import { user } from "$/store/auth";
	import { graphql } from "$/gql";
	import { CombinedError, getContextClient } from "@urql/svelte";
	import ShieldX from "$/components/icons/settings/shield-x.svelte";
	import Dialog from "$/components/dialog.svelte";
	import Spinner from "$/components/spinner.svelte";
	import { FieldStatusType, type FieldStatus } from "$/components/form/field.svelte";
	import PasswordField from "$/components/form/password-field.svelte";
	import { fieldsValid } from "$/lib/utils";

	const dispatch = createEventDispatcher();
	const client = getContextClient();

	let passwordStatus: FieldStatus;
	let password: string;

	$: formValid = fieldsValid([passwordStatus]);

	let loading = false;

	function isWrongPassword(err: CombinedError) {
		return (
			Array.isArray(err.graphQLErrors[0].extensions.fields) &&
			err.graphQLErrors[0].extensions.fields.includes("password")
		);
	}

	async function disableTotp() {
		if (formValid) {
			loading = true;
			const res = await client
				.mutation(
					graphql(`
						mutation DisableTotp($password: String!) {
							user {
								twoFa {
									resp: disableTotp(password: $password) {
										totpEnabled
									}
								}
							}
						}
					`),
					{
						password,
					},
					{
						requestPolicy: "network-only",
					},
				)
				.toPromise();
			loading = false;
			if (res.data && $user) {
				$user.totpEnabled = res.data.user.twoFa.resp.totpEnabled;
				close();
			} else if (res.error && isWrongPassword(res.error)) {
				passwordStatus = { type: FieldStatusType.Error, message: "Wrong password" };
			}
		}
	}

	function close() {
		if (!loading) {
			dispatch("close");
		}
	}
</script>

<Dialog on:close={close}>
	<h1 class="heading">
		<ShieldX size={1.8 * 16} />
		<span>Disable 2FA</span>
	</h1>
	<p class="text">Please confirm your password before disabling 2-Factor-Authentication.</p>
	<form id="disable-2fa-form" on:submit|preventDefault={disableTotp}>
		<PasswordField
			label="Password"
			autocomplete="current-password"
			required
			bind:value={password}
			bind:status={passwordStatus}
		/>
	</form>
	<div class="buttons">
		<button class="button secondary" on:click={close} disabled={loading}>Cancel</button>
		<button
			class="button primary submit"
			type="submit"
			form="disable-2fa-form"
			disabled={loading || !formValid}
		>
			{#if loading}
				<Spinner />
			{/if}
			Disable
		</button>
	</div>
</Dialog>

<style lang="scss">
	@import "../../../assets/styles/variables.scss";

	.heading {
		font-size: 1.8rem;

		display: flex;
		align-items: center;
		gap: 0.5rem;

		& > span {
			font-size: 2rem;
		}
	}

	.text {
		font-weight: 500;
		color: $textColorLight;
	}

	.buttons {
		display: flex;
		align-items: center;
		gap: 1rem;
		justify-content: flex-end;

		& > .button {
			padding: 0.4rem 0.8rem;
		}
	}

	.button.submit {
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}
</style>
