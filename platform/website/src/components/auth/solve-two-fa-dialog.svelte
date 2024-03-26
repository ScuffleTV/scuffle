<script lang="ts">
	import { getContextClient } from "@urql/svelte";
	import { graphql } from "$/gql";
	import { sessionToken, currentTwoFaRequest } from "$/store/auth";
	import { z } from "zod";
	import Field, { FieldStatusType, type FieldStatus } from "../form/field.svelte";
	import Dialog from "../dialog.svelte";

	const client = getContextClient();

	export let requestId: string;

	let codeStatus: FieldStatus;
	let codeLabel = "Code";
	let code: string;
	async function codeValidate(v: string) {
		const valid = z
			.string()
			.regex(/^(\d{6}|[0-9a-fA-F]{8})$/, "Invalid code")
			.safeParse(v);

		codeLabel = "Code";
		if (!valid.success) {
			return { type: FieldStatusType.Error, message: valid.error.issues[0].message };
		}
		// If it is a backup code
		if (/^[0-9a-fA-F]{8}$/.test(v)) {
			codeLabel = "Backup Code";
		}
		return { type: FieldStatusType.None };
	}

	async function onSubmit() {
		const res = await client
			.mutation(
				graphql(`
					mutation SolveTwoFa($id: ULID!, $code: String!) {
						auth {
							resp: fulfillTwoFaRequest(id: $id, code: $code) {
								__typename
								... on Session {
									token
								}
							}
						}
					}
				`),
				{ id: requestId, code: code },
				{
					requestPolicy: "network-only",
				},
			)
			.toPromise();
		if (res.data) {
			codeStatus = { type: FieldStatusType.Success };
			if (res.data.auth.resp?.__typename === "Session") {
				$sessionToken = res.data.auth.resp.token;
			}
			closeDialog();
		} else if (res.error) {
			if (
				Array.isArray(res.error.graphQLErrors[0].extensions.fields) &&
				res.error.graphQLErrors[0].extensions.fields.includes("code") &&
				typeof res.error.graphQLErrors[0].extensions.reason === "string"
			) {
				codeStatus = {
					type: FieldStatusType.Error,
					message: res.error.graphQLErrors[0].extensions.reason,
				};
			}
		}
	}

	function closeDialog() {
		$currentTwoFaRequest = null;
	}
</script>

<Dialog on:close={closeDialog}>
	<h2>2FA Challenge</h2>
	<p class="details-text">
		Please enter the code from your authenticator app. In case you don't have access to your code
		anymore, please enter a backup code.
	</p>
	<form on:submit|preventDefault={onSubmit}>
		<Field
			label={codeLabel}
			autocomplete="one-time-code"
			bind:value={code}
			validate={codeValidate}
			bind:status={codeStatus}
		/>
		<div class="button-group">
			<input class="button-submit" type="submit" value="Submit" />
		</div>
	</form>
</Dialog>

<style lang="scss">
	@import "../../assets/styles/variables.scss";

	form {
		text-align: center;
	}

	.button-group {
		display: flex;
		flex-direction: column;
		justify-content: center;
		align-items: center;
		margin-top: 1rem;
	}

	.details-text {
		color: $textColorLight;
	}

	.button-submit {
		border: none;
		cursor: pointer;
		border-radius: 0.5rem;
		color: $textColor;
		font: inherit;

		width: 45%;
		font-size: 1rem;
		font-weight: 400;
		padding: 0.8rem;
		background-color: $primaryColor;
		transition:
			background-color 0.5s,
			color 0.5s,
			box-shadow 0.5s;
		box-shadow: 0px 6px 20px 7px rgba(255, 115, 87, 0.1);

		&:hover:not(:disabled),
		&:focus-visible:not(:disabled) {
			background-color: $primaryColorLight;
			box-shadow: 0px 6px 20px 7px rgba(255, 115, 87, 0.2);
		}

		&:disabled {
			background-color: $primaryColorDark;
			box-shadow: 0px 6px 20px 7px rgba(255, 115, 87, 0.05);
			cursor: not-allowed;
			color: $textColorLight;
		}
	}
</style>
