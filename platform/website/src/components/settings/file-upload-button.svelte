<script lang="ts">
	import { graphql } from "$/gql";
	import { FileStatus } from "$/gql/graphql";
	import { uploadFile } from "$/lib/fileUpload";
	import { PUBLIC_CF_TURNSTILE_KEY, PUBLIC_UPLOAD_ENDPOINT } from "$env/static/public";
	import { faUpload } from "@fortawesome/free-solid-svg-icons";
	import { getContextClient } from "@urql/svelte";
	import { createEventDispatcher } from "svelte";
	import Fa from "svelte-fa";
	import { Turnstile } from "svelte-turnstile";
	import { pipe, subscribe, type Subscription } from "wonka";
	import ErrorDialog from "../error-dialog.svelte";

	export let endpoint: string;
	export let disabled: boolean = false;
	export let pendingFileId: string | null = null;

	const dispatch = createEventDispatcher();
	const client = getContextClient();

	let files: FileList;
	let input: HTMLInputElement;
	let turnstileToken: string | null = null;

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
					pendingFileId = null;
					if (data.fileStatus.status === FileStatus.Failure) {
						console.error("file upload failed: ", data.fileStatus.reason);
						fileError = data.fileStatus.friendlyMessage ?? data.fileStatus.reason ?? null;
						dispatch("error");
					} else {
						dispatch("success");
					}
				}
			}),
		);
	}

	$: subToFileStatus(pendingFileId);

	function uploadProfilePicture() {
		if (turnstileToken) {
			uploadFile(
				`${PUBLIC_UPLOAD_ENDPOINT}/${endpoint}`,
				{ set_active: true },
				files[0],
				turnstileToken,
			)
				.then((res) => res.json())
				.then((res) => {
					if (res.success) {
						pendingFileId = res.file_id ?? null;
						dispatch("pending");
					} else {
						fileError = res.message ?? null;
						pendingFileId = null;
						dispatch("error");
					}
				})
				.catch((err) => {
					fileError = err;
					pendingFileId = null;
					dispatch("error");
				});
		}
	}

	$: if (files && files[0]) {
		dispatch("uploading");
		uploadProfilePicture();
	}
</script>

{#if fileError}
	<ErrorDialog heading="Failed to upload" message={fileError} on:close={() => (fileError = null)} />
{/if}

<!-- Putting sr-only here to prevent it from showing but still render it. aria-hidden is true to make the screenreader ignore the element. -->
<div class="sr-only" aria-hidden="true">
	<Turnstile
		appearance="interaction-only"
		siteKey={PUBLIC_CF_TURNSTILE_KEY}
		on:turnstile-callback={(e) => (turnstileToken = e.detail.token)}
	/>
</div>

<!-- Pseudo button that clicks the hidden input -->
<button
	class="button primary"
	on:click={() => input.click()}
	disabled={!turnstileToken || !!pendingFileId || disabled}
>
	<Fa icon={faUpload} />
	<slot />
</button>
<input
	type="file"
	accept="image/webp, image/avif, image/avif-sequence, image/gif, image/png, image/apng, image/jls, image/jpeg, image/jxl, image/bmp, image/heic, image/heic-sequence, image/heif, image/heif-sequence, application/mp4, video/mp4, video/x-flv, video/x-matroska, video/avi, video/quicktime, video/webm, video/mp2t"
	name="file"
	bind:this={input}
	bind:files
	hidden
/>

<style lang="scss">
	@import "../../assets/styles/settings.scss";
</style>
