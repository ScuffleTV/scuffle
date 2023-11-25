<script lang="ts">
	import Dialog from "$/components/dialog.svelte";
	import { FieldStatusType, type FieldStatus } from "$/components/form/field.svelte";
	import PasswordField from "$/components/form/password-field.svelte";
	import Spinner from "$/components/spinner.svelte";
	import { graphql } from "$/gql";
	import { fieldsValid, passwordValidate } from "$/lib/utils";
	import { currentTwoFaRequest } from "$/store/auth";
	import { faEdit } from "@fortawesome/free-solid-svg-icons";
	import { CombinedError, getContextClient } from "@urql/svelte";
	import { createEventDispatcher } from "svelte";
	import Fa from "svelte-fa";

	const dispatch = createEventDispatcher();
	const client = getContextClient();

	let loading = false;

	let currentPasswordStatus: FieldStatus;
	let currentPassword: string;

	let newPasswordStatus: FieldStatus;
	let newPassword: string;

	let confirmPasswordStatus: FieldStatus;
	let confirmPassword: string;

	$: formValid = fieldsValid([currentPasswordStatus, newPasswordStatus, confirmPasswordStatus]);

	async function newPasswordValidate(v: string) {
		if (confirmPassword === v) {
			confirmPasswordStatus = { type: FieldStatusType.Success };
		} else {
			confirmPasswordStatus = { type: FieldStatusType.Error, message: "Passwords do not match" };
		}
		return await passwordValidate(v);
	}

	async function confirmPasswordValidate(v: string) {
		if (v !== newPassword) {
			return { type: FieldStatusType.Error, message: "Passwords do not match" };
		}
		return { type: FieldStatusType.Success };
	}

	function isWrongPassword(err: CombinedError) {
		return (
			Array.isArray(err.graphQLErrors[0].extensions.fields) &&
			err.graphQLErrors[0].extensions.fields.includes("password")
		);
	}

	async function changePassword() {
		if (formValid && currentPassword && newPassword) {
			loading = true;
			const res = await client
				.mutation(
					graphql(`
						mutation ChangePassword($currentPassword: String!, $newPassword: String!) {
							user {
								resp: password(currentPassword: $currentPassword, newPassword: $newPassword) {
									__typename
									... on TwoFaRequest {
										id
									}
								}
							}
						}
					`),
					{
						currentPassword,
						newPassword,
					},
					{
						requestPolicy: "network-only",
					},
				)
				.toPromise();
			loading = false;
			if (res.data) {
				if (res.data.user.resp?.__typename === "TwoFaRequest") {
					$currentTwoFaRequest = res.data.user.resp.id;
				}
				close();
			} else if (res.error && isWrongPassword(res.error)) {
				currentPasswordStatus = { type: FieldStatusType.Error, message: "Wrong password" };
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
		<Fa icon={faEdit} />
		<span>Change Password</span>
	</h1>
	<!-- TODO: Replace forget password link -->
	<p class="text">
		Please confirm your current password before changing it.
		<a href="/forgot-password">Forgot your password?</a>
	</p>
	<form id="change-password-form" on:submit|preventDefault={changePassword}>
		<PasswordField
			label="Current Password"
			autocomplete="current-password"
			required
			bind:value={currentPassword}
			bind:status={currentPasswordStatus}
		/>
		<PasswordField
			label="New Password"
			autocomplete="new-password"
			required
			bind:value={newPassword}
			validate={newPasswordValidate}
			bind:status={newPasswordStatus}
		/>
		<PasswordField
			label="Confirm New Password"
			autocomplete="new-password"
			required
			bind:value={confirmPassword}
			validate={confirmPasswordValidate}
			bind:status={confirmPasswordStatus}
		/>
	</form>
	<div class="buttons">
		<button class="button secondary" on:click={close} disabled={loading}>Cancel</button>
		<button
			class="button primary submit"
			type="submit"
			form="change-password-form"
			disabled={loading || !formValid}
		>
			{#if loading}
				<Spinner />
			{/if}
			Change
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

		a {
			color: $primaryColor;
			text-decoration: none;

			&:hover,
			&:focus-visible {
				text-decoration: underline;
			}
		}
	}

	form {
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}

	.buttons {
		margin-top: 1rem;

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
