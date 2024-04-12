import type { HslColor } from "$/gql/graphql";

export function colorToStyle(color?: HslColor, darkMode: boolean = true) {
	if (!color) return "";
	const s = `${color.s * 100}%`;
	let l;
	if (darkMode) {
		l = Math.max(color.l, 0.5);
	} else {
		l = Math.min(color.l, 0.8);
	}
	l = `${l * 100}%`;
	return `color: hsl(${color.h}, ${s}, ${l});`;
}

export function rgbHexToHsl(color?: string) {
	if (!color) return;
	const [r, g, b] = color.match(/\w\w/g)?.map((x) => parseInt(x, 16)) ?? [];
	if (r === undefined || g === undefined || b === undefined) return;
	return rgbToHsl(r, g, b);
}

// https://www.rapidtables.com/convert/color/rgb-to-hsl.html
function rgbToHsl(r: number, g: number, b: number) {
	r /= 255;
	g /= 255;
	b /= 255;

	const cMax = Math.max(r, g, b);
	const cMin = Math.min(r, g, b);
	const delta = cMax - cMin;

	let h = 0;

	if (delta !== 0) {
		switch (cMax) {
			case r:
				h = 60 * (((g - b) / delta) % 6);
				break;
			case g:
				h = 60 * ((b - r) / delta + 2);
				break;
			case b:
				h = 60 * ((r - g) / delta + 4);
				break;
		}
	}

	if (h < 0) h += 360;

	const l = (cMax + cMin) / 2;

	let s = 0;
	if (delta !== 0) {
		s = delta / (1 - Math.abs(2 * l - 1));
	}

	return {
		h,
		s,
		l,
	};
}
