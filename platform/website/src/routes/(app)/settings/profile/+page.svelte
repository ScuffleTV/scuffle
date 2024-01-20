<script lang="ts">
	import Color from "$/components/settings/profile/color.svelte";
	import { user } from "$/store/auth";
	import { faPalette, faTrashAlt } from "@fortawesome/free-solid-svg-icons";
	import { graphql } from "$gql";
	import Fa from "svelte-fa";
	import { getContextClient } from "@urql/svelte";
	import Section from "$/components/settings/section.svelte";
	import StatusBar, { Status } from "$/components/settings/status-bar.svelte";
	import SectionContainer from "$/components/settings/section-container.svelte";
	import Field, { FieldStatusType, type FieldStatus } from "$/components/form/field.svelte";
	import ProfilePicture from "$/components/user/profile-picture.svelte";
	import Spinner from "$/components/spinner.svelte";
	import { colorToStyle, rgbHexToHsl } from "$/lib/colors";
	import FileUploadButton from "$/components/settings/file-upload-button.svelte";
	import ResponsiveImage from "$/components/responsive-image.svelte";

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

	$: status =
		displayName !== $user?.displayName || displayColorRgb !== $user?.displayColor.rgb
			? Status.Changed
			: Status.Unchanged;

	function resetStatus() {
		status = Status.Unchanged;
	}

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
</script>

{#if $user}
	<SectionContainer>
		<Section title="Profile Picture" details="Personalize your account with a profile picture.">
			<div class="input big">
				{#if $user.pendingProfilePictureId}
					<div class="picture-pending">
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
					<FileUploadButton endpoint="profile-picture" bind:pendingFileId={$user.pendingProfilePictureId} on:error={resetStatus} on:success={resetStatus} on:pending={resetStatus} on:uploading={() => (status = Status.Saving)}>Upload Picture</FileUploadButton>
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
		<Section title="Offline Banner" details="Personalize your account with a offline banner.">
			<div class="input big offline-banner">
				{#if $user.channel.pendingOfflineBannerId}
					<div class="picture-pending">
						<Spinner />
					</div>
				{:else}
					{#if $user.channel.offlineBanner}
						<ResponsiveImage image={$user.channel.offlineBanner} alt="offline banner" aspectRatio="5/1" width="100%" fitMode="cover" />
					{:else}
						Not Set
					{/if}
				{/if}
				<div class="buttons">
					<FileUploadButton endpoint="offline-banner" bind:pendingFileId={$user.channel.pendingOfflineBannerId} on:error={resetStatus} on:success={resetStatus} on:pending={resetStatus} on:uploading={() => (status = Status.Saving)}>Upload Picture</FileUploadButton>
					<button
						class="button secondary"
						disabled={!$user.channel.offlineBanner}
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

	.input.big.offline-banner {
		display: grid;
		grid-template-columns: 1fr;
	}

	.picture-pending {
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
