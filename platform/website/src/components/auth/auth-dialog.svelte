<script lang="ts">
	import { Turnstile } from "svelte-turnstile";
	import { AuthDialog, authDialog, currentTwoFaRequest, sessionToken } from "$/store/auth";
	import { z } from "zod";
	import { getContextClient } from "@urql/svelte";
	import { graphql } from "$gql";
	import { PUBLIC_CF_TURNSTILE_KEY } from "$env/static/public";
	import Dialog from "../dialog.svelte";
	import { FieldStatusType, type FieldStatus, resetAllFields } from "../form/field.svelte";
	import Field from "$/components/form/field.svelte";
	import PasswordField from "../form/password-field.svelte";
	import { fieldsValid, passwordValidate } from "$/lib/utils";

	const client = getContextClient();

	const loginQuery = graphql(`
		mutation Login($username: String!, $password: String!, $captchaToken: String!) {
			auth {
				resp: login(username: $username, password: $password, captchaToken: $captchaToken) {
					__typename
					... on Session {
						token
					}
					... on TwoFaRequest {
						id
					}
				}
			}
		}
	`);
	const registerQuery = graphql(`
		mutation Register(
			$username: String!
			$password: String!
			$email: String!
			$captchaToken: String!
		) {
			auth {
				resp: register(
					username: $username
					password: $password
					email: $email
					captchaToken: $captchaToken
				) {
					token
				}
			}
		}
	`);

	let globalMessage = "";
	let globalIsError = false;
	let turnstileToken = "";
	let loggingIn = false;

	$: $authDialog, resetAllFields();

	let usernameValue: string;
	let usernameStatus: FieldStatus;
	let usernameValidationTimeout: number | NodeJS.Timeout;
	let rejectValidation: () => void;
	async function usernameValidate(v: string) {
		globalMessage = "";
		if ($authDialog === AuthDialog.Login) {
			return { type: FieldStatusType.Success };
		}

		clearTimeout(usernameValidationTimeout);
		if (rejectValidation) {
			rejectValidation();
		}

		const valid = z
			.string()
			.min(3, "Minimum of 3 characters")
			.max(20, "Maximum of 20 characters")
			.regex(/^[a-zA-Z0-9_]*$/, "Username can only contain letters, numbers, and underscores")
			.safeParse(v);
		if (!valid.success) {
			return { type: FieldStatusType.Error, message: valid.error.issues[0].message };
		}

		usernameStatus = {
			type: FieldStatusType.Loading,
			message: "Hold on while we check if this username is available...",
		};

		// Wait 500 milliseconds before checking the username. This prevents spamming the server with requests.
		await new Promise((resolve, reject) => {
			rejectValidation = reject;
			usernameValidationTimeout = setTimeout(resolve, 500);
		});

		const result = await client
			.query(
				graphql(`
					query CheckUsername($username: String!) {
						user {
							user: byUsername(username: $username) {
								id
							}
						}
					}
				`),
				{ username: v },
				{
					requestPolicy: "network-only",
				},
			)
			.toPromise();

		if (result.error) {
			return { type: FieldStatusType.Error, message: "Something went wrong." };
		}

		if (result.data?.user.user?.id) {
			return { type: FieldStatusType.Error, message: "This username is already taken" };
		}

		return { type: FieldStatusType.Success };
	}

	let emailStatus: FieldStatus;
	let emailValue: string;
	async function emailValidate(v: string) {
		globalMessage = "";
		const valid = z.string().email("Please enter a valid email").safeParse(v);
		return valid.success
			? { type: FieldStatusType.Success }
			: { type: FieldStatusType.Error, message: valid.error.issues[0].message };
	}

	let passwordStatus: FieldStatus;
	let passwordValue: string;
	async function authPasswordValidate(v: string) {
		globalMessage = "";
		return await passwordValidate(v);
	}

	let confirmPasswordStatus: FieldStatus;
	async function confirmPasswordValidate(v: string) {
		globalMessage = "";
		if (passwordValue !== v) {
			return { type: FieldStatusType.Error, message: "Passwords do not match" };
		}

		return { type: FieldStatusType.Success };
	}

	$: formValid =
		turnstileToken.length > 0 &&
		($authDialog === AuthDialog.Login
			? fieldsValid([usernameStatus, passwordStatus])
			: fieldsValid([usernameStatus, emailStatus, passwordStatus, confirmPasswordStatus]));

	function closeDialog() {
		$authDialog = AuthDialog.Closed;
	}

	function toggleMode() {
		if ($authDialog === AuthDialog.Login) {
			$authDialog = AuthDialog.Register;
		} else {
			$authDialog = AuthDialog.Login;
		}
	}

	function clearTurnstileToken() {
		turnstileToken = "";
	}

	function onTurnstileCallback(event: CustomEvent<{ token: string }>) {
		turnstileToken = event.detail.token;
	}

	/// This function is only ever called from the onSubmit event of the form.
	async function handleSubmit() {
		const request =
			$authDialog === AuthDialog.Login
				? {
						query: loginQuery,
						variables: {
							username: usernameValue,
							password: passwordValue,
							captchaToken: turnstileToken,
						},
				  }
				: {
						query: registerQuery,
						variables: {
							username: usernameValue,
							password: passwordValue,
							email: emailValue,
							captchaToken: turnstileToken,
						},
				  };

		const response = await client
			.mutation(request.query, request.variables, {
				requestPolicy: "network-only",
			})
			.toPromise();

		if (response.error) {
			globalIsError = true;
			const error = response.error.graphQLErrors[0];
			if (error?.extensions?.kind === "InvalidInput") {
				let fields = (error.extensions?.fields as string[]) || [];
				globalMessage = `${error.extensions.reason || error.message}`;
				for (const field of fields) {
					if (field === "username") {
						usernameStatus = { type: FieldStatusType.Error };
					} else if (field === "password") {
						passwordStatus = { type: FieldStatusType.Error };
					} else if (field === "email") {
						emailStatus = { type: FieldStatusType.Error };
					} else if (field == "captchaToken") {
						turnstileToken = "";
					}
				}
			} else {
				globalMessage = "An unknown error occured, if the problem persists please contact support";
				console.error("Bad GQL response", response);
			}

			let turnstileID = document.querySelector("#login-turnstile-container iframe")?.id;
			if (turnstileID) window.turnstile.reset(turnstileID);
			else {
				globalMessage = "An unknown error occured, please refresh the page and try again";
				console.error("Could not find turnstile iframe");
			}

			return;
		}

		if (response.data?.auth.resp.__typename === "TwoFaRequest") {
			$currentTwoFaRequest = response.data?.auth.resp.id;
			closeDialog();
		} else if (
			response.data?.auth.resp.__typename === "Session" &&
			response.data?.auth.resp.token
		) {
			globalIsError = false;
			globalMessage = "Success!";
			$sessionToken = response.data?.auth.resp.token;
			closeDialog();
		} else {
			globalIsError = true;
			globalMessage = "An unknown error occured, if the problem persists please contact support";
			console.error("Bad GQL response", response);
		}
	}

	// This is the function that is called when the form is submitted.
	async function onSubmit() {
		if (!formValid) {
			return;
		}

		// This prevents the user from spamming the submit button. The button becomes disabled preventing further submissions.
		// However we still check since there are other ways to submit the form. (On mobile, or weird browsers)
		if (loggingIn) {
			return;
		}

		loggingIn = true;

		// This function is a wrapper because we have early returns, and we want to make sure the button is re-enabled.
		try {
			await handleSubmit();
		} catch (e) {
			globalIsError = true;
			globalMessage = "An unknown error occured, if the problem persists please contact support";
			console.error("Exception during submit: ", e);
		}

		loggingIn = false;
	}
</script>

<Dialog on:close={closeDialog}>
	<div class="login-title">
		<h2 class="text-left signup-title">
			{$authDialog === AuthDialog.Login ? "Login" : "Sign up"}
		</h2>
		<h2 class="text-left signup-subtitle">
			{$authDialog === AuthDialog.Login ? "Don't have an account?" : "Already have an account?"}
			<span>
				<button class="link-button" on:click={toggleMode} role="link">
					{$authDialog === AuthDialog.Login ? "Sign up" : "Sign in"}
				</button>
			</span>
		</h2>
	</div>
	<form on:submit|preventDefault={onSubmit}>
		<Field
			type="text"
			autocomplete="username"
			required
			label="Username"
			bind:status={usernameStatus}
			bind:value={usernameValue}
			validate={usernameValidate}
		/>
		{#if $authDialog === AuthDialog.Register}
			<Field
				type="email"
				autocomplete="email"
				required
				label="Email"
				bind:status={emailStatus}
				bind:value={emailValue}
				validate={emailValidate}
			/>
		{/if}
		<PasswordField
			autocomplete={$authDialog === AuthDialog.Login ? "current-password" : "new-password"}
			required
			label="Password"
			bind:status={passwordStatus}
			bind:value={passwordValue}
			validate={authPasswordValidate}
		/>
		{#if $authDialog === AuthDialog.Register}
			<PasswordField
				autocomplete="new-password"
				required
				label="Confirm Password"
				bind:status={confirmPasswordStatus}
				validate={confirmPasswordValidate}
			/>
		{/if}
		{#if globalMessage}
			<div class="message-holder" class:error={globalIsError}>
				<span>{globalMessage}</span>
			</div>
		{/if}
		<div id="login-turnstile-container">
			<Turnstile
				siteKey={PUBLIC_CF_TURNSTILE_KEY}
				on:turnstile-callback={onTurnstileCallback}
				on:turnstile-error={clearTurnstileToken}
				on:turnstile-expired={clearTurnstileToken}
				on:turnstile-timeout={clearTurnstileToken}
			/>
		</div>
		<div class="button-group">
			<input
				class="button-submit"
				type="submit"
				value={loggingIn ? "Loading..." : $authDialog === AuthDialog.Login ? "Login" : "Sign up"}
				disabled={!formValid || loggingIn}
				aria-disabled={!formValid || loggingIn}
			/>
		</div>
	</form>
</Dialog>

<style lang="scss">
	@import "../../assets/styles/variables.scss";

	form {
		text-align: center;
	}

	.link-button {
		color: $primaryColor;
		font-size: inherit;
		background-color: transparent;
		margin: 0;
		padding: 0;
		border: 0;
		cursor: pointer;
		transition: color 0.2s ease-in-out;

		&:hover,
		&:focus-visible {
			color: $primaryColorLight;
		}
	}

	.text-left {
		text-align: left !important;
	}

	.signup-title {
		font-weight: 600;
		font-size: 2rem;
	}

	.login-title {
		margin-bottom: 2.2rem;
	}

	.signup-subtitle {
		font-weight: 400;
		margin-top: 0.5rem;
		font-size: 0.95rem;
		color: $textColorDark;
	}

	form {
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}

	.button-group {
		display: flex;
		flex-direction: column;
		justify-content: center;
		align-items: center;
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

	.message-holder {
		margin-bottom: 0.5rem;
		color: $successColor;
		&.error {
			color: $errorColor;
		}
	}

	#login-turnstile-container {
		height: 4.5rem;
	}
</style>
