<script lang="ts">
	import { onDestroy, onMount } from "svelte";
	import { Turnstile } from "svelte-turnstile";
	import { AuthDialog, authDialog, sessionToken } from "$/store/auth";
	import LoginField, { newField } from "$/components/auth/field.svelte";
	import { z } from "zod";
	import { getContextClient } from "@urql/svelte";
	import { graphql } from "$gql";
	import MouseTrap from "$components/mouse-trap.svelte";
	import { PUBLIC_CF_TURNSTILE_KEY } from "$env/static/public";
	import { user } from "$/store/auth";
	import { fade } from "svelte/transition";
	import type { User } from "$/gql/graphql";

	const client = getContextClient();

	const loginQuery = graphql(`
		mutation Login($username: String!, $password: String!, $captchaToken: String!) {
			auth {
				resp: login(username: $username, password: $password, captchaToken: $captchaToken) {
					token
					user {
						id
						displayName
						displayColor {
							color
							hue
							isGray
						}
						username
						email
						emailVerified
						lastLoginAt
						channel {
							id
							liveViewerCount
						}
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
					user {
						id
						displayName
						displayColor {
							color
							hue
							isGray
						}
						username
						email
						emailVerified
						lastLoginAt
						channel {
							id
							liveViewerCount
						}
					}
				}
			}
		}
	`);

	let globalMessage = "";
	let globalIsError = false;
	let turnstileToken = "";
	let loggingIn = false;

	let username = newField({
		id: "username",
		type: "text",
		label: "Username",
		autoComplete: "username",
		extra: {
			timeout: undefined as NodeJS.Timeout | undefined,
		},
		reload() {
			username.touched = username.value.length > 0;

			if ($authDialog === AuthDialog.Login) {
				username.message = "";
				username.status = "";
			}

			if (username.touched) {
				username.update(username.value);
			}
		},
		update(value) {
			username.value = value;

			// We clear the timeout so that we dont send a request to the server on every character change.
			clearTimeout(username.extra.timeout);

			// When we are in login mode, we dont need to validate the username.
			if ($authDialog !== AuthDialog.Register) {
				return;
			}

			const valid = z
				.string()
				.min(3, "Minimum of 3 characters")
				.max(20, "Maximum of 20 characters")
				.regex(/^[a-zA-Z0-9_]*$/, "Username can only contain letters, numbers, and underscores")
				.safeParse(username.value);
			if (!valid.success) {
				username.status = "error";
				username.message = valid.error.issues[0].message;
				return;
			}

			username.message = "Hold on while we check if this username is available...";
			username.status = "loading";

			// This is known as a debouncer.
			// We dont want to send a request to the server on every character change.
			// Instead we wait 200ms after the user stops typing before sending the request.
			username.extra.timeout = setTimeout(async () => {
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
						{ username: username.value },
						{
							requestPolicy: "network-only",
						},
					)
					.toPromise();

				if (result.error) {
					username.status = "error";
					username.message = "Something went wrong.";
					return;
				}

				if (result.data?.user.user?.id) {
					username.status = "error";
					username.message = "This username is already taken";
					return;
				}

				username.status = "success";
				username.message = "";
			}, 200);
		},
		valid() {
			// We cannot refer to the value stored at `username.value` directly here since we are using it to return a value.
			// This causes typescript to complain that the value is referenced in the value definition.
			// To get around this we need to unfortunately cast the value to a string so that typescript knows it is a string before the value `username` value is defined.
			// This is safe since we know that the value is a string.
			const value = username.value as string;
			const status = username.status as string;

			if ($authDialog === AuthDialog.Login) {
				return value.length > 0;
			}

			return status === "success";
		},
		validate(value) {
			// This is on character change validation (not on submit)
			return z
				.string()
				.regex(/^[a-zA-Z0-9_]*$/, "Username can only contain letters, numbers, and underscores")
				.safeParse(value).success;
		},
	});

	let email = newField({
		id: "email",
		type: "email",
		label: "Email",
		autoComplete: "email",
		reload() {
			email.touched = email.value.length > 0;
			if (email.touched) {
				email.update(email.value);
			}
		},
		update(value) {
			email.value = value;

			if ($authDialog !== AuthDialog.Register) {
				return;
			}

			const valid = z.string().email("Please enter a valid email").safeParse(email.value);
			if (!valid.success) {
				email.status = "error";
				email.message = valid.error.issues[0].message;
				return;
			}

			email.status = "success";
			email.message = "";
		},
		valid() {
			const status = email.status as string;
			return status === "success";
		},
	});

	let password = newField({
		id: "password",
		type: "password",
		label: "Password",
		reload() {
			password.touched = password.value.length > 0;
			password.status = "";
			password.message = "";

			password.autoComplete =
				$authDialog === AuthDialog.Login ? "current-password" : "new-password";

			password.update(password.value);
		},
		update(value) {
			password.value = value;

			// When we are in login mode, we dont need to validate the password.
			if ($authDialog !== AuthDialog.Register) {
				return;
			}

			// Since the validity of confirm password depends on the password, we need to cause a re-render of the confirm password field.
			confirmPassword.update(confirmPassword.value);

			const valid = z
				.string()
				.min(8, "Minimum of 8 characters")
				.max(32, "Maximum of 32 chracters")
				.regex(/.*[A-Z].*/, "Atleast One uppercase character")
				.regex(/.*[a-z].*/, "Atleast One lowercase character")
				.regex(/.*\d.*/, "Atleast One number")
				.regex(/.*[`~<>?,./!@#$%^&*()\-_+="'|{}[\];:].*/, "One special character")
				.safeParse(value);

			if (!valid.success) {
				password.status = "error";
				password.message = valid.error.issues[0].message;
				return;
			}

			password.status = "success";
			password.message = "";
		},
		valid() {
			const value = password.value as string;
			const status = password.status as string;

			if ($authDialog === AuthDialog.Login) {
				return value.length > 0;
			}

			return status === "success";
		},
	});

	let confirmPassword = newField({
		id: "confirmPassword",
		type: "password",
		label: "Confirm Password",
		autoComplete: "new-password",
		reload() {
			confirmPassword.touched = confirmPassword.value.length > 0;
			confirmPassword.status = "";
			confirmPassword.message = "";
		},
		update(value) {
			confirmPassword.value = value;

			if ($authDialog !== AuthDialog.Register) {
				return;
			}

			if (password.value !== confirmPassword.value) {
				confirmPassword.status = "error";
				confirmPassword.message = "Passwords do not match";
				return;
			}

			if (confirmPassword.value.length === 0) {
				confirmPassword.status = "";
				confirmPassword.message = "";
				return;
			}

			confirmPassword.status = "success";
			confirmPassword.message = "";
		},
		valid() {
			const status = confirmPassword.status as string;

			return status === "success";
		},
	});

	$: formValid =
		($authDialog === AuthDialog.Login
			? username.valid() && password.valid()
			: username.valid() && email.valid() && password.valid() && confirmPassword.valid()) &&
		turnstileToken.length > 0;

	// When the login mode changes we need to reload the fields.
	// This is because the fields have specific logic for each mode.
	let unsubscribe = authDialog.subscribe((m) => {
		if (m === AuthDialog.Closed) {
			return;
		}

		username.reload();
		email.reload();
		password.reload();
		confirmPassword.reload();
	});

	onDestroy(unsubscribe);

	function closeDialog() {
		$authDialog = AuthDialog.Closed;
	}

	function handleKeyDown(event: KeyboardEvent) {
		if (event.key === "Escape") {
			closeDialog();
		}
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
							username: username.value,
							password: password.value,
							captchaToken: turnstileToken,
						},
				  }
				: {
						query: registerQuery,
						variables: {
							username: username.value,
							password: password.value,
							email: email.value,
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
						username.status = "error";
						username.message = "";
					} else if (field === "password") {
						password.status = "error";
						password.message = "";
					} else if (field === "email") {
						email.status = "error";
						email.message = "";
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

		const token = response.data?.auth.resp.token;
		const userData = response.data?.auth.resp.user;
		if (!token || !userData) {
			globalIsError = true;
			globalMessage = "An unknown error occured, if the problem persists please contact support";
			console.error("Bad GQL response", response);
			return;
		}

		globalIsError = false;
		globalMessage = "Success!";
		$sessionToken = token;
		$user = userData as User;
		closeDialog();
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

	let dialog: HTMLDialogElement;

	onMount(() => dialog.showModal());
</script>

<svelte:window on:keydown={handleKeyDown} />

<dialog
	bind:this={dialog}
	class="popup"
	aria-label={$authDialog === AuthDialog.Login ? "Login popup" : "Sign up popup"}
	aria-modal="true"
	in:fade={{ duration: 100 }}
	out:fade={{ duration: 300 }}
>
	<MouseTrap on:close={closeDialog}>
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
			<LoginField field={username} />
			{#if $authDialog === AuthDialog.Register}
				<LoginField field={email} />
			{/if}
			<LoginField field={password} />
			{#if $authDialog === AuthDialog.Register}
				<LoginField field={confirmPassword} />
			{/if}
			{#if globalMessage !== ""}
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
	</MouseTrap>
</dialog>

<style lang="scss">
	@import "../../assets/styles/variables.scss";

	.popup {
		width: min(30rem, 90vw);
		background: linear-gradient(to bottom, #18191a, #101415);

		border-radius: 0.25rem;
		padding: 2.5rem;
		box-shadow: 0 0 0.5rem 0.5rem rgba(0, 0, 0, 0.3);
		border: none;

		color: $textColor;
		text-align: center;

		display: flex;
		flex-direction: column;

		&::backdrop {
			background-color: rgba(0, 0, 0, 0.5);
		}
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
	}

	.button-group {
		display: flex;
		flex-direction: column;
		justify-content: center;
		align-items: center;
		margin-top: 1rem;
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
