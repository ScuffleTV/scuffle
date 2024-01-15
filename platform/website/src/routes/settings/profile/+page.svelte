<script lang="ts">
	import Color from "$/components/settings/profile/color.svelte";
	import { user } from "$/store/auth";
	import { faPalette, faTrashAlt, faUpload } from "@fortawesome/free-solid-svg-icons";
	import { graphql } from "$gql";
	import Fa from "svelte-fa";
	import { getContextClient } from "@urql/svelte";
	import Section from "$/components/settings/section.svelte";
	import StatusBar, { Status } from "$/components/settings/status-bar.svelte";
	import SectionContainer from "$/components/settings/section-container.svelte";
	import Field, { FieldStatusType, type FieldStatus } from "$/components/form/field.svelte";
	import { uploadFile } from "$/lib/fileUpload";
	import { PUBLIC_CF_TURNSTILE_KEY, PUBLIC_UPLOAD_ENDPOINT } from "$env/static/public";
	import { Turnstile } from "svelte-turnstile";
	import ProfilePicture from "$/components/user/profile-picture.svelte";
	import Spinner from "$/components/spinner.svelte";
	import { pipe, subscribe, type Subscription } from "wonka";
	import { FileStatus } from "$/gql/graphql";
	import ErrorDialog from "$/components/error-dialog.svelte";

	const recommendedColors = ["#ff7a00", "#ffe457", "#57ff86", "#00ffd1", "#5786ff", "#8357ff"];

	const client = getContextClient();

	let status = Status.Unchanged;

	let displayNameStatus: FieldStatus;
	let displayName = $user?.displayName;
	async function displayNameValidate(v: string) {
		if (v.toLowerCase() !== $user?.displayName.toLowerCase()) {
			return { type: FieldStatusType.Error, message: "You may only change capatilization" };
		}
		return { type: FieldStatusType.Success };
	}

	let displayColor = $user?.displayColor.color;
	let displayColorInput: HTMLInputElement;

	let avatarFiles: FileList;
	let avatarInput: HTMLInputElement;

	$: status =
		displayName !== $user?.displayName || displayColor !== $user?.displayColor.color
			? Status.Changed
			: Status.Unchanged;

	function saveChanges() {
		if (displayName !== $user?.displayName) {
			saveDisplayName();
		}
		if (displayColor !== $user?.displayColor.color) {
			saveDisplayColor();
		}
	}

	function saveDisplayName() {
		if (displayName) {
			status = Status.Saving;
			const request = {
				query: graphql(`
					mutation SetDisplayName($displayName: String!) {
						user {
							resp: displayName(displayName: $displayName) {
								displayName
							}
						}
					}
				`),
				variables: {
					displayName,
				},
			};
			client
				.mutation(request.query, request.variables, {
					requestPolicy: "network-only",
				})
				.toPromise()
				.then((result) => {
					if (result.data) {
						displayName = result.data.user.resp.displayName;
						if ($user) {
							$user.displayName = result.data.user.resp.displayName;
						}
					} else if (result.error && $user) {
						displayName = $user.displayName;
					}
				});
		}
	}

	function saveDisplayColor() {
		if (displayColor) {
			status = Status.Saving;
			const request = {
				query: graphql(`
					mutation SetDisplayColor($color: Color!) {
						user {
							resp: displayColor(color: $color) {
								displayColor {
									color
									hue
									isGray
								}
							}
						}
					}
				`),
				variables: {
					color: displayColor,
				},
			};
			client
				.mutation(request.query, request.variables, {
					requestPolicy: "network-only",
				})
				.toPromise()
				.then((result) => {
					if (result.data) {
						displayColor = result.data.user.resp.displayColor.color;
						if ($user) {
							$user.displayColor = result.data.user.resp.displayColor;
						}
					} else if (result.error && $user) {
						displayColor = $user.displayColor.color;
					}
				});
		}
	}

	let fileSub: Subscription;
	let fileError: string | null = null;

	function subToFileStatus(fileId?: string | null) {
		fileSub?.unsubscribe();
		if (!fileId) return;
		fileSub = pipe(
			client.subscription(
				graphql(`
					subscription FileStatus($fileId: ULID!) {
						fileStatus(fileId: $fileId) {
							fileId
							status
							reason
							friendlyMessage
						}
					}
				`),
				{ fileId },
			),
			subscribe(({ data }) => {
				if (data) {
					if (data.fileStatus.status === FileStatus.Failure) {
						console.error("file upload failed: ", data.fileStatus.reason);
						fileError = data.fileStatus.friendlyMessage ?? data.fileStatus.reason ?? null;
					}
					if ($user) $user.pendingProfilePictureId = null;
				}
			}),
		);
	}

	$: subToFileStatus($user?.pendingProfilePictureId);

	let turnstileToken: string | null = null;

	function uploadProfilePicture() {
		if (turnstileToken) {
			uploadFile(
				`${PUBLIC_UPLOAD_ENDPOINT}/profile-picture`,
				{ set_active: true },
				avatarFiles[0],
				turnstileToken,
			)
				.then((res) => res.json())
				.then((res) => {
					status = Status.Unchanged;
					if (res.success) {
						if ($user) $user.pendingProfilePictureId = res.file_id ?? null;
					} else {
						fileError = res.message ?? null;
					}
				})
				.catch((err) => {
					fileError = err;
					status = Status.Unchanged;
				});
		}
	}

	function removeProfilePicture() {
		client
			.mutation(
				graphql(`
					mutation RemoveProfilePicture {
						user {
							resp: removeProfilePicture {
								profilePicture {
									id
								}
							}
						}
					}
				`),
				{},
				{ requestPolicy: "network-only" },
			)
			.toPromise()
			.then(({ data }) => {
				if (data && $user) {
					$user.pendingProfilePictureId = null;
					$user.profilePicture = null;
				}
			});
	}

	$: {
		if (avatarFiles && avatarFiles[0]) {
			status = Status.Saving;
			uploadProfilePicture();
		}
	}
</script>

{#if $user}
	{#if fileError}
		<ErrorDialog
			heading="Failed to upload"
			message={fileError}
			on:close={() => (fileError = null)}
		/>
	{/if}
	<SectionContainer>
		<Section title="Profile Picture" details="Personalize your account with a profile picture.">
			<!-- Putting sr-only here to prevent it from showing but still render it. aria-hidden is true to make the screenreader ignore the element. -->
			<div class="sr-only" aria-hidden="true">
				<Turnstile
					appearance="interaction-only"
					siteKey={PUBLIC_CF_TURNSTILE_KEY}
					on:turnstile-callback={(e) => (turnstileToken = e.detail.token)}
				/>
			</div>
			<div class="input big">
				{#if $user.pendingProfilePictureId}
					<div class="profile-picture-pending">
						<Spinner />
					</div>
				{:else}
					<ProfilePicture
						userId={$user.id}
						displayColor={$user.displayColor}
						profilePicture={$user.profilePicture}
						size={6 * 16}
					/>
				{/if}
				<div class="buttons">
					<!-- Pseudo button that clicks the hidden input -->
					<button
						class="button primary"
						on:click={() => avatarInput.click()}
						disabled={!turnstileToken || !!$user.pendingProfilePictureId}
					>
						<Fa icon={faUpload} />
						Upload Picture
					</button>
					<input
						type="file"
						accept="image/webp, image/avif, image/avif-sequence, image/gif, image/png, image/apng, image/jls, image/jpeg, image/jxl, image/bmp, image/heic, image/heic-sequence, image/heif, image/heif-sequence, application/mp4, video/mp4, video/x-flv, video/x-matroska, video/avi, video/quicktime, video/webm, video/mp2t"
						name="file"
						bind:this={avatarInput}
						bind:files={avatarFiles}
						hidden
					/>
					<button
						class="button secondary"
						on:click={removeProfilePicture}
						disabled={!$user.profilePicture}
					>
						<Fa icon={faTrashAlt} />
						Remove Picture
					</button>
				</div>
			</div>
		</Section>
		<Section
			title="Display Name"
			details="What shows up as your channel name."
			showReset={displayName !== $user.displayName}
			on:reset={() => (displayName = $user?.displayName)}
		>
			<Field
				type="text"
				autocomplete="username"
				placeholder="Display Name"
				bind:value={displayName}
				validate={displayNameValidate}
				bind:status={displayNameStatus}
			/>
		</Section>
		<Section
			title="Display Color"
			details="The color of your name in chat."
			showReset={displayColor !== $user.displayColor.color}
			on:reset={() => (displayColor = $user?.displayColor.color)}
		>
			<div class="input big display-color">
				<span class="display-name" style="color: {displayColor}">{$user?.displayName}</span>
				<div class="color-picker">
					<div class="colors">
						{#each recommendedColors as color}
							<Color {color} on:click={() => (displayColor = color)} />
						{/each}
					</div>
					<!-- Pseudo button that clicks the hidden input -->
					<button class="button primary" on:click={() => displayColorInput.click()}>
						<Fa icon={faPalette} />
						Choose Color
					</button>
					<input type="color" bind:this={displayColorInput} bind:value={displayColor} hidden />
				</div>
			</div>
		</Section>
		<StatusBar
			{status}
			on:save={saveChanges}
			saveDisabled={displayNameStatus?.type === FieldStatusType.Error}
		/>
	</SectionContainer>
{/if}

<style lang="scss">
	@import "../../../assets/styles/variables.scss";
	@import "../../../assets/styles/settings.scss";

	.profile-picture-pending {
		display: flex;
		justify-content: center;
		align-items: center;
		height: 6rem;
		width: 6rem;
		border-radius: 50%;
		background-color: $bgColorLight;
	}

	.input.display-color {
		& > .display-name {
			text-align: center;
			font-weight: 500;
			font-size: 1.25rem;
			flex-grow: 1;
		}
	}

	.color-picker {
		display: flex;
		flex-direction: column;
		gap: 1rem;

		& > .colors {
			display: flex;
			flex-wrap: wrap;
			gap: 0.5rem;

			max-width: 20rem;
		}
	}
</style>
