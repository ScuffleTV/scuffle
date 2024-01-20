<script lang="ts">
	import { ImageUploadFormat, type ImageUpload, type ImageUploadVariant } from "$/gql/graphql";

	// Disclaimer: When you want to understand this code in detail, you should probably make yourself familiar with responsive images first. It's quite a complex topic.
	// https://css-tricks.com/a-guide-to-the-responsive-images-syntax-in-html
	// https://developer.mozilla.org/en-US/docs/Learn/HTML/Multimedia_and_embedding/Responsive_images

	export let image: ImageUpload;
	export let width: string | number | undefined = undefined;
	export let height: string | number | undefined = undefined;
	export let fitMode: "contain" | "cover" = "contain";
	export let alt: string = "";
	export let rounded: boolean = false;
	export let background: boolean = false;
	// This prop should be set to the estimated width the image will have on the full page.
	export let fullPageWidth: string | null = null;

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
					// Check if this image is in width mode or scale mode
					if (fullPageWidth) {
						return res + `${image.endpoint}/${a.url} ${a.width}w, `;
					} else {
						return res + `${image.endpoint}/${a.url} ${a.scale}x, `;
					}
				}, "")
				.slice(0, -2);
			grouped[i].srcSet = srcSet;
		}

		return {
			bestSupported,
			variants: grouped,
		};
	}

	$: preparedVariants = prepareVariants(image.variants);
</script>

{#if preparedVariants && preparedVariants.bestSupported}
	<picture>
		{#each preparedVariants.variants as variant}
			<source
				type={variant.type}
				srcset={variant.srcSet}
				sizes={fullPageWidth}
				width={typeof width === "number" ? width : null}
				height={typeof height === "number" ? height : null}
				media={variant.media}
			/>
		{/each}
		<img
			class:rounded
			class:background
			src="{image.endpoint}/{preparedVariants.bestSupported.url}"
			width={typeof width === "number" ? width : null}
			height={typeof height === "number" ? height : null}
			style:width={typeof width !== "number" ? width : null}
			style:height={typeof height !== "number" ? height : null}
			style:object-fit={fitMode}
			{alt}
		/>
	</picture>
{/if}

<style lang="scss">
	@import "../assets/styles/variables.scss";

	picture {
		line-height: 0;
	}

	img {
		background-color: $bgColorLight;

		&.rounded {
			border-radius: 50%;
		}

		&.background {
			pointer-events: none;
			position: absolute;
			top: 0;
			left: 0;
			bottom: 0;
			right: 0;

			z-index: -1;
		}
	}
</style>
