<script lang="ts">
    import { graphql } from "$/gql";
    import { pipe, subscribe, type Subscription } from "wonka";
	import { ImageUploadFormat, type DisplayColor, type ImageUpload, type ImageUploadVariant } from "$/gql/graphql";
    import { getContextClient } from "@urql/svelte";
	import { onDestroy } from "svelte";
	import { websocketOpen } from "$/store/websocket";
	import DefaultAvatar from "./default-avatar.svelte";
	import { groupBy } from "$/lib/utils";

    export let userId: string;
    export let profilePicture: ImageUpload | null | undefined;
	export let displayColor: DisplayColor;
	export let size: number = 48;

	// From least supported to best supported
	const FORMAT_SORT_ORDER = [
		ImageUploadFormat.Avif,
		ImageUploadFormat.AvifStatic,
		ImageUploadFormat.Webp,
		ImageUploadFormat.WebpStatic,
		ImageUploadFormat.Gif,
		ImageUploadFormat.PngStatic,
	];

	// Sorts the variants by scale and format
	function sortVariants(variants?: ImageUploadVariant[]) {
		if (!variants) return [];
		return Object.values(groupBy(variants, (v) => FORMAT_SORT_ORDER.indexOf(v.format))).map((v) => v.sort((a, b) => a.scale - b.scale));
	}

	function variantsToSrcset(variants: ImageUploadVariant[]) {
		return variants.reduce((res, a) => {
			return res + `${profilePicture?.endpoint}/${a.url} ${a.scale}x, `;
		}, "").slice(0, -2);
	}

	$: variants = sortVariants(profilePicture?.variants);

	$: console.log("variants", variants);

	// Finds the best supported image variant
	// First looks for a gif, then a static png
	function bestSupported(variants: ImageUploadVariant[]) {
		const gif = variants.find((v) => v.format === ImageUploadFormat.Gif);
		if (gif) return gif;
		return variants.find((v) => v.format === ImageUploadFormat.PngStatic);
	}

	$: bestSupportedVariant = profilePicture ? bestSupported(profilePicture.variants) : undefined;

	function formatToMimeType(format: ImageUploadFormat) {
		switch (format) {
			case ImageUploadFormat.Avif:
			case ImageUploadFormat.AvifStatic:
				return "image/avif";
			case ImageUploadFormat.Gif:
				return "image/gif";
			case ImageUploadFormat.PngStatic:
				return "image/png";
			case ImageUploadFormat.Webp:
				case ImageUploadFormat.WebpStatic:
				return "image/webp";
		}
	}

    const client = getContextClient();

	let sub: Subscription;

	function subToProfilePicture(userId: string) {
		sub?.unsubscribe();
		sub = pipe(
			client.subscription(
				graphql(`
					subscription ProfilePicture($userId: ULID!) {
						userProfilePicture(userId: $userId) {
							profilePicture {
                                id
                                variants {
									width
									height
                                    scale
                                    url
									format
									byteSize
                                }
								endpoint
                            }
						}
					}
				`),
				{ userId },
			),
			subscribe(({ data }) => {
				if (data) {
					profilePicture = data.userProfilePicture.profilePicture;
				}
			}),
		);
	}

	$: if ($websocketOpen) {
		subToProfilePicture(userId);
	}

	$: if (!$websocketOpen) {
		sub?.unsubscribe();
	}

	onDestroy(() => {
		sub?.unsubscribe();
	});
</script>

{#if profilePicture && bestSupportedVariant && variants}
	<picture>
		{#each variants as variantsOfFormat}
			<source type={formatToMimeType(variantsOfFormat[0].format)} srcset={variantsToSrcset(variantsOfFormat)} width={size} height={size} />
		{/each}
		<img class="avatar" src="{profilePicture.endpoint}/{bestSupportedVariant.url}" width={size} height={size} alt="avatar" />
	</picture>
{:else}
    <DefaultAvatar {userId} {displayColor} {size} />
{/if}

<style lang="scss">
	@import "../../assets/styles/variables.scss";

	picture {
		line-height: 0;
		background-color: $bgColorLight;
		border-radius: 50%;
	}

    .avatar {
        border-radius: 50%;
    }
</style>
