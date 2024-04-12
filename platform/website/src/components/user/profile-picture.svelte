<script lang="ts">
	import { graphql } from "$/gql";
	import { pipe, subscribe, type Subscription } from "wonka";
	import {
		ImageUploadFormat,
		type DisplayColor,
		type ImageUpload,
		type ImageUploadVariant,
	} from "$/gql/graphql";
	import { getContextClient } from "@urql/svelte";
	import { onDestroy } from "svelte";
	import { websocketOpen } from "$/store/websocket";
	import DefaultAvatar from "./default-avatar.svelte";

	export let userId: string;
	export let profilePicture: ImageUpload | null | undefined;
	export let displayColor: DisplayColor;
	export let size: number = 48;

	// From least supported to best supported
	const FORMAT_SORT_ORDER = [
		ImageUploadFormat.AvifStatic,
		ImageUploadFormat.WebpStatic,
		ImageUploadFormat.PngStatic,

		ImageUploadFormat.Avif,
		ImageUploadFormat.Webp,
		ImageUploadFormat.Gif,
	];

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

	function isFormatStatic(format?: ImageUploadFormat) {
		return (
			format === ImageUploadFormat.AvifStatic ||
			format === ImageUploadFormat.WebpStatic ||
			format === ImageUploadFormat.PngStatic
		);
	}

	// This function prepares the variants for the <picture> element by grouping them by format, sorting them by scale and generating the required media and srcSet tags.
	// It also returns the best supported variant for use in the fallback <img> element which is the smallest GIF or PNG.
	function prepareVariants(variants?: ImageUploadVariant[]): {
		bestSupported: ImageUploadVariant | null;
		variants: { type: string; srcSet: string; media: string }[];
	} {
		if (!variants) return { bestSupported: null, variants: [] };

		const animated = variants.some((v) => !isFormatStatic(v.format));

		variants.sort((a, b) => a.scale - b.scale);

		const grouped: {
			type: string;
			srcSet: string;
			media: string;
			variants: ImageUploadVariant[];
		}[] = Object.values(
			variants.reduce(
				(res, v) => {
					const format = FORMAT_SORT_ORDER.indexOf(v.format);
					if (!res[format]) {
						// Always true
						let media = "(min-width: 0px)";
						if (isFormatStatic(v.format) && animated) {
							media += " and (prefers-reduced-motion: reduce)";
						}
						res[format] = { type: formatToMimeType(v.format), srcSet: "", media, variants: [] };
					}
					res[format].variants.push(v);
					return res;
				},
				{} as {
					[key: number]: {
						type: string;
						srcSet: string;
						media: string;
						variants: ImageUploadVariant[];
					};
				},
			),
		);

		const bestSupported =
			grouped[FORMAT_SORT_ORDER.indexOf(ImageUploadFormat.Gif)]?.variants[0] ??
			grouped[FORMAT_SORT_ORDER.indexOf(ImageUploadFormat.PngStatic)]?.variants[0] ??
			null;

		// add srcset
		for (let i = 0; i < grouped.length; i++) {
			const srcSet = grouped[i].variants
				.reduce((res, a) => {
					return res + `${profilePicture?.endpoint}/${a.url} ${a.scale}x, `;
				}, "")
				.slice(0, -2);
			grouped[i].srcSet = srcSet;
		}

		return {
			bestSupported,
			variants: grouped,
		};
	}

	$: preparedVariants = prepareVariants(profilePicture?.variants);

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

{#if profilePicture && preparedVariants && preparedVariants.bestSupported}
	<picture>
		{#each preparedVariants.variants as variant}
			<source
				type={variant.type}
				srcset={variant.srcSet}
				width={size}
				height={size}
				media={variant.media}
			/>
		{/each}
		<img
			class="avatar"
			src="{profilePicture.endpoint}/{preparedVariants.bestSupported.url}"
			width={size}
			height={size}
			alt="avatar"
		/>
	</picture>
{:else}
	<DefaultAvatar {userId} {displayColor} {size} />
{/if}

<style lang="scss">
	@import "../../assets/styles/variables.scss";

	picture {
		line-height: 0;
	}

	.avatar {
		border-radius: 50%;
		background-color: $bgColorLight;
	}
</style>
