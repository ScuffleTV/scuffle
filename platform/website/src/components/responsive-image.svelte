<script lang="ts">
	import { ImageUploadFormat, type ImageUploadVariant } from "$/gql/graphql";

    export let variants: ImageUploadVariant[];
    export let endpoint: string;
    export let size: number | undefined = undefined;
    export let alt: string = "";
    export let rounded: boolean = false;
    export let background: boolean = false;

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
					return res + `${endpoint}/${a.url} ${a.scale}x, `;
				}, "")
				.slice(0, -2);
			grouped[i].srcSet = srcSet;
		}

		return {
			bestSupported,
			variants: grouped,
		};
	}

	$: preparedVariants = prepareVariants(variants);
</script>

{#if preparedVariants && preparedVariants.bestSupported}
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
            class:rounded={rounded}
            class:background={background}
            src="{endpoint}/{preparedVariants.bestSupported.url}"
            width={size}
            height={size}
            alt={alt}
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

            width: 100%;
            height: 100%;
            z-index: -1;
            object-fit: cover;
        }
    }
</style>
