<script lang="ts">
	import Dialog from "$/components/dialog.svelte";
	import Field, { FieldStatusType, type FieldStatus } from "$/components/form/field.svelte";
	import ShieldCheck from "$/components/icons/settings/shield-check.svelte";
	import Spinner from "$/components/spinner.svelte";
	import { graphql } from "$/gql";
	import { downloadBackupCodes } from "$/lib/twoFa";
	import { user } from "$/store/auth";
	import { faArrowRight, faDownload } from "@fortawesome/free-solid-svg-icons";
	import { CombinedError, getContextClient } from "@urql/svelte";
	import { createEventDispatcher } from "svelte";
	import Fa from "svelte-fa";
	import { z } from "zod";

	const dispatch = createEventDispatcher();
	const client = getContextClient();

	function clickClose() {
		if (state.step === 3) {
			next(false);
		} else {
			dispatch("close");
		}
	}

	function next(fromForm: boolean) {
		if (state.step === 1) {
			generateSecret();
		} else if (state.step === 2 && fromForm) {
			enableTotp();
		} else if (state.step === 3) {
			downloadBackupCodes(backupCodes);
			dispatch("close");
		}
	}

	let backupCodes: string[];

	async function generateSecret() {
		loading = true;
		const res = await client
			.mutation(
				graphql(`
					mutation GenerateSecret {
						user {
							twoFa {
								resp: generateTotp {
									qrCode
									backupCodes
								}
							}
						}
					}
				`),
				{},
				{
					requestPolicy: "network-only",
				},
			)
			.toPromise();
		loading = false;
		if (res.data && $user) {
			backupCodes = res.data.user.twoFa.resp.backupCodes;
			// Show QR code
			state = {
				step: 2,
				qrCode: res.data.user.twoFa.resp.qrCode,
			};
		}
	}

	function isWrongCode(err: CombinedError) {
		return (
			Array.isArray(err.graphQLErrors[0].extensions.fields) &&
			err.graphQLErrors[0].extensions.fields.includes("code")
		);
	}

	async function enableTotp() {
		if (codeStatus.type !== FieldStatusType.Success) {
			return;
		}
		loading = true;
		const res = await client
			.mutation(
				graphql(`
					mutation EnableTotp($code: String!) {
						user {
							twoFa {
								resp: enableTotp(code: $code) {
									totpEnabled
								}
							}
						}
					}
				`),
				{
					code,
				},
				{
					requestPolicy: "network-only",
				},
			)
			.toPromise();
		loading = false;
		if (res.data && $user) {
			state = { step: 3 };
			$user.totpEnabled = res.data.user.twoFa.resp.totpEnabled;
		} else if (res.error && isWrongCode(res.error)) {
			codeStatus = { type: FieldStatusType.Error, message: "Invalid code" };
		}
	}

	let state: { step: 1 } | { step: 2; qrCode: string } | { step: 3 } = { step: 1 };

	let loading = false;

	let codeStatus: FieldStatus;
	let code: string;

	async function codeValidate(v: string) {
		const valid = z
			.string()
			.length(6, "Code must be 6 digits")
			.regex(/^\d{6}$/, "Invalid code")
			.safeParse(v);

		if (!valid.success) {
			return { type: FieldStatusType.Error, message: valid.error.issues[0].message };
		}

		return { type: FieldStatusType.Success };
	}
</script>

<Dialog on:close={clickClose} width={35}>
	<div class="title-container">
		<h1 class="heading">
			<ShieldCheck size={1.8 * 16} />
			<span>Enable 2FA</span>
		</h1>
		<span class="step">Step {state.step} of 3</span>
	</div>
	{#if state.step === 1}
		<p class="text">
			2-Factor-Authentication adds more security to your account by asking for a code from a
			third-party authenticator app (i.e. Google Authenticator, Twilio Authy, etc.) when logging in.
		</p>
	{:else if state.step === 2}
		<form class="step-2" id="enable-2fa-step-2" on:submit|preventDefault={() => next(true)}>
			<img src="data:image/png;base64,{state.qrCode}" width="200" height="200" alt="TOTP QR Code" />
			<div>
				<span class="text">
					Please scan the QR code with your authenticator app and submit the code.
				</span>
				<Field
					type="text"
					label="Code"
					autocomplete="one-time-code"
					required
					bind:value={code}
					validate={codeValidate}
					bind:status={codeStatus}
				/>
			</div>
		</form>
	{:else if state.step === 3}
		<p class="text">
			2FA has been enabled successfully. Please store your backup codes in a secure place. Please be
			aware that, if you lose your backup codes, you may get locked out of your account.
		</p>
	{/if}
	<div class="buttons">
		{#if state.step !== 3}
			<button class="button secondary" on:click={clickClose}>Cancel</button>
		{/if}
		<button class="button primary next" form="enable-2fa-step-2" on:click={() => next(false)}>
			{#if loading}
				<Spinner />
			{/if}
			{#if state.step === 3}
				<Fa icon={faDownload} />
				Download Backup Codes
			{:else}
				Next
				<Fa icon={faArrowRight} />
			{/if}
		</button>
	</div>
</Dialog>

<style lang="scss">
	@import "../../../assets/styles/variables.scss";

	.title-container {
		display: flex;
		justify-content: space-between;
		align-items: center;
	}

	.heading {
		font-size: 1.8rem;

		display: flex;
		align-items: center;
		gap: 0.5rem;

		& > span {
			font-size: 2rem;
		}
	}

	.step {
		color: $textColorLight;
	}

	.text {
		font-weight: 500;
		color: $textColorLight;
	}

	.step-2 {
		margin: 1rem 0;

		display: flex;
		gap: 2rem;
		justify-content: center;

		& > div {
			display: flex;
			flex-direction: column;
			gap: 1rem;
		}
	}

	@media screen and (max-width: $mobileBreakpoint) {
		.step-2 {
			flex-wrap: wrap;
		}
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

	.button.next {
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}
</style>
