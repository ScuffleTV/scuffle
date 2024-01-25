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
	import { colorToStyle, rgbHexToHsl } from "$/lib/colors";
	import ImageEditorDialog from "$/components/settings/image-editor-dialog.svelte";

	const imageEditorTypes = ["image/png", "image/jls", "image/jpeg", "image/jxl", "image/bmp"];

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

	let displayColorRgb = $user?.displayColor.rgb;
	let displayColorInput: HTMLInputElement;

	let avatarFiles: FileList | null = null;
	let avatarSrc: string | null = null;
	let avatarInput: HTMLInputElement;

	$: status =
		displayName !== $user?.displayName || displayColorRgb !== $user?.displayColor.rgb
			? Status.Changed
			: Status.Unchanged;

	function saveChanges() {
		if (displayName !== $user?.displayName) {
			saveDisplayName();
		}
		if (displayColorRgb !== $user?.displayColor.rgb) {
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
		if (displayColorRgb) {
			status = Status.Saving;
			const request = {
				query: graphql(`
					mutation SetDisplayColor($color: RgbColor!) {
						user {
							resp: displayColor(color: $color) {
								displayColor {
									rgb
									hsl {
										h
										s
										l
									}
									isGray
								}
							}
						}
					}
				`),
				variables: {
					color: displayColorRgb,
				},
			};
			client
				.mutation(request.query, request.variables, {
					requestPolicy: "network-only",
				})
				.toPromise()
				.then((result) => {
					if (result.data) {
						displayColorRgb = result.data.user.resp.displayColor.rgb;
						if ($user) {
							$user.displayColor = result.data.user.resp.displayColor;
						}
					} else if (result.error && $user) {
						displayColorRgb = $user.displayColor.rgb;
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

	function uploadProfilePicture(blob: Blob | null) {
		if (turnstileToken && blob) {
			status = Status.Saving;
			uploadFile(
				`${PUBLIC_UPLOAD_ENDPOINT}/profile-picture`,
				{ set_active: true },
				blob,
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
			resetAvatarFile();
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
			if (imageEditorTypes.includes(avatarFiles[0].type)) {
				const reader = new FileReader();
				reader.onload = (e) => {
					if (!e.target) return;
					avatarSrc = e.target.result as string;
				};

				reader.readAsDataURL(avatarFiles[0]);
			} else {
				uploadProfilePicture(avatarFiles[0]);
			}
		}
	}

	function resetAvatarFile() {
		avatarFiles = null;
		avatarSrc = null;
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
	{#if avatarSrc}
		<ImageEditorDialog
			src={avatarSrc}
			on:close={resetAvatarFile}
			on:submit={(e) => uploadProfilePicture(e.detail.blob)}
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
			showReset={displayColorRgb !== $user.displayColor.rgb}
			on:reset={() => (displayColorRgb = $user?.displayColor.rgb)}
		>
			<div class="input big display-color">
				<span class="display-name" style={colorToStyle(rgbHexToHsl(displayColorRgb))}
					>{$user?.displayName}</span
				>
				<div class="color-picker">
					<div class="colors">
						{#each recommendedColors as color}
							<Color rgb={color} on:click={() => (displayColorRgb = color)} />
						{/each}
					</div>
					<!-- Pseudo button that clicks the hidden input -->
					<button class="button primary" on:click={() => displayColorInput.click()}>
						<Fa icon={faPalette} />
						Choose Color
					</button>
					<input type="color" bind:this={displayColorInput} bind:value={displayColorRgb} hidden />
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
	@import "../../../../assets/styles/variables.scss";
	@import "../../../../assets/styles/settings.scss";

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
