<script lang="ts">
	import { getContextClient } from "@urql/svelte";
	import Field, { newField } from "./field.svelte";
	import { graphql } from "$/gql";
	import { authDialog, session, AuthDialog } from "$/store/auth";
	import { z } from "zod";

	const client = getContextClient();

	const code = newField({
		id: "code",
		type: "text",
		label: "Code",
		autoComplete: "one-time-code",
		update(value) {
			code.value = value;

			const valid = z
				.string()
				.regex(/^(\d{6}|[0-9a-fA-F]{8})$/, "Invalid code")
				.safeParse(code.value);

			code.label = "Code";
			if (valid.success) {
				code.status = "";
				code.message = "";
				// If it is a backup code
				if (/^[0-9a-fA-F]{8}$/.test(code.value)) {
					code.label = "Backup Code";
				}
			} else {
				code.status = "error";
				code.message = valid.error.issues[0].message;
			}
		},
		valid() {
			const status = code.status as string;
			return status === "success";
		},
		validate(value) {
			return /^[0-9a-fA-F]*$/.test(value);
		},
	});

	async function onSubmit() {
		const res = await client
			.mutation(
				graphql(`
					mutation SolveTwoFa($code: String!) {
						auth {
							resp: verifyTotpCode(code: $code) {
								token
								twoFaSolved
							}
						}
					}
				`),
				{ code: code.value },
				{
					requestPolicy: "network-only",
				},
			)
			.toPromise();
		if (res.data) {
			code.status = "success";
			code.message = "";
			$session = res.data.auth.resp;
			$authDialog = AuthDialog.Closed;
		} else if (res.error) {
			if (
				Array.isArray(res.error.graphQLErrors[0].extensions.fields) &&
				res.error.graphQLErrors[0].extensions.fields.includes("code") &&
				typeof res.error.graphQLErrors[0].extensions.reason === "string"
			) {
				code.status = "error";
				code.message = res.error.graphQLErrors[0].extensions.reason;
			}
		}
	}
</script>

<h2>2FA Challenge</h2>
<p class="details-text">
	Please enter the code from your authenticator app. In case you don't have access to your code
	anymore, please enter a backup code.
</p>
<form on:submit|preventDefault={onSubmit}>
	<Field field={code} />
	<div class="button-group">
		<input class="button-submit" type="submit" value="Submit" />
	</div>
</form>

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
