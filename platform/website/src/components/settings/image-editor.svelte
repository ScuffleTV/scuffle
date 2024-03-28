<script lang="ts">
	// Don't touch this when you don't know what you're doing
	// You've gotta do the 5Head math shit to understand most of this

	// When you actually want to understand this, here are a few acronyms I used
	//
	// Normalized space:
	// xl: x left
	// xr: x right
	// yt: y top
	// yb: y bottom
	//
	// Real Pixel Space:
	// rw: real width
	// rh: real height
	// rd: real dimension
	// xrl: x real left
	// yrt: y real top
	// ox: offset x
	// oy: offset y

	export let overlay = true;
	export let gridOverlay = false;
	export let size = 40 * 16;
	// between minScale and maxScale
	export let scale = 1;
	export let minScale: number;
	export let maxScale: number;
	export let src: string;

	let mouseStartX: number;
	let mouseStartY: number;
	let moveable: HTMLImageElement;
	let aspectRatio: number = 0;

	// x and y are the normalized coordinates of the top left corner of the unscaled image (scale=1)
	// In normalized space (0,0) is left/top and (1,1) is right/bottom
	let x = 0;
	let y = 0;

	$: applyLimits(), scale;

	// If the image is currently grabbed
	let moving = false;

	function applyLimits() {
		if (!moveable) return;

		const width = moveable.naturalWidth / Math.min(moveable.naturalWidth, moveable.naturalHeight);
		const height = moveable.naturalHeight / Math.min(moveable.naturalWidth, moveable.naturalHeight);

		const minLimitX = 1 - width * scale;
		const minLimitY = 1 - height * scale;
		const maxLimit = 0;

		const sX = (width * (scale - 1)) / 2;
		const sY = (height * (scale - 1)) / 2;
		const xl = x - sX;
		const yt = y - sY;

		// Limit movement to parent element
		if (xl > maxLimit) {
			x = maxLimit + sX;
		} else if (xl < minLimitX) {
			x = minLimitX + sX;
		}
		if (yt > maxLimit) {
			y = maxLimit + sY;
		} else if (yt < minLimitY) {
			y = minLimitY + sY;
		}
	}

	function reset() {
		// Reset scale
		scale = 1;
		// Reset position
		center();
		applyLimits();
	}

	// Center the image on the viewport
	function center() {
		if (moveable) {
			const width = moveable.naturalWidth / Math.min(moveable.naturalWidth, moveable.naturalHeight);
			const height =
				moveable.naturalHeight / Math.min(moveable.naturalWidth, moveable.naturalHeight);
			x = -(width / 2 - 0.5);
			y = -(height / 2 - 0.5);
		} else {
			x = 0;
			y = 0;
		}
	}

	function mouseDown(e: MouseEvent) {
		e.preventDefault();
		const xs = x * size;
		const ys = y * size;
		mouseStartX = e.clientX - xs;
		mouseStartY = e.clientY - ys;
		moving = true;
	}

	function mouseMove(e: MouseEvent) {
		if (!moving) return;

		e.preventDefault();

		// Update element position
		x = (e.clientX - mouseStartX) / size;
		y = (e.clientY - mouseStartY) / size;

		applyLimits();
	}

	function mouseUp() {
		moving = false;
	}

	function wheel(e: WheelEvent) {
		e.preventDefault();

		scale += e.deltaY * -0.001;
		scale = Math.min(Math.max(minScale, scale), maxScale);
	}

	export function calculateResult(callback: BlobCallback) {
		if (!moveable) return;

		const canvas = document.createElement("canvas");
		const ctx = canvas.getContext("2d");

		if (!ctx) return;

		const width = moveable.naturalWidth / Math.min(moveable.naturalWidth, moveable.naturalHeight);
		const height = moveable.naturalHeight / Math.min(moveable.naturalWidth, moveable.naturalHeight);

		const sX = (width * (scale - 1)) / 2;
		const sY = (height * (scale - 1)) / 2;
		const xl = x - sX;
		const yt = y - sY;

		const rw = moveable.naturalWidth / (scale * width);
		const rh = moveable.naturalHeight / (scale * height);
		const xrl = -xl * rw;
		const yrt = -yt * rh;

		const rd = Math.min(rw, rh);
		const ox = (rd - rw) / 2;
		const oy = (rd - rh) / 2;

		canvas.width = rd;
		canvas.height = rd;

		ctx.drawImage(moveable, xrl, yrt, rw, rh, ox, oy, rw, rh);

		canvas.toBlob(callback, "image/png");
	}

	function updateAspectRatio() {
		aspectRatio = moveable?.naturalWidth / moveable?.naturalHeight;
		reset();
		applyLimits();
	}
</script>

<svelte:window on:mousemove={mouseMove} on:mouseup={mouseUp} />

<div class="wrapper" style="--size: {size}px">
	<!-- svelte-ignore a11y-no-noninteractive-element-interactions -->
	<img
		class="moveable"
		{src}
		bind:this={moveable}
		draggable="false"
		on:mousedown={mouseDown}
		on:wheel={wheel}
		on:load={updateAspectRatio}
		class:wide={aspectRatio > 1}
		class:high={aspectRatio < 1}
		style="--scale: {scale}; --x: {x * size}px; --y: {y * size}px"
		alt="upload a file"
	/>
	{#if overlay}
		<div class="mask"></div>
	{/if}
	{#if gridOverlay}
		<div class="grid">
			<div></div>
			<div class="y-axis"></div>
			<div></div>
			<div class="x-axis"></div>
			<div class="center"></div>
			<div class="x-axis"></div>
			<div></div>
			<div class="y-axis"></div>
			<div></div>
		</div>
	{/if}
</div>

<style lang="scss">
	@import "../../assets/styles/variables.scss";

	.wrapper {
		width: var(--size);
		height: var(--size);
		position: relative;
		background-color: $bgColor;

		&:not(.debug) {
			overflow: hidden;
		}
	}

	.mask {
		position: absolute;
		top: 0;
		left: 0;
		bottom: 0;
		right: 0;
		background: radial-gradient(transparent 70.5%, rgba(0, 0, 0, 0.5) 70.5%);
		pointer-events: none;

		&:after {
			content: "";
			position: absolute;
			top: 0;
			left: 0;
			bottom: 0;
			right: 0;
			border: 2px solid white;
			border-radius: 50%;
			pointer-events: none;
		}
	}

	.grid {
		position: absolute;
		top: 0;
		left: 0;
		bottom: 0;
		right: 0;
		pointer-events: none;
		display: grid;
		grid-template-columns: repeat(3, 1fr);
		grid-template-rows: repeat(3, 1fr);

		& > * {
			border-style: solid;
			border-color: white;
			border-width: 0;
		}

		& > .y-axis {
			border-left-width: 1px;
			border-right-width: 1px;
		}

		& > .x-axis {
			border-top-width: 1px;
			border-bottom-width: 1px;
		}

		& > .center {
			border-width: 1px;
		}
	}

	.moveable {
		position: absolute;
		cursor: move;
		transform-origin: center;
		transform: translate(var(--x), var(--y)) scale(var(--scale));
		min-width: 100%;
		min-height: 100%;
		max-width: 100%;
		max-height: 100%;

		&.wide {
			min-width: 100%;
			max-width: unset;
			max-height: 100%;
		}

		&.high {
			min-height: 100%;
			max-width: 100%;
			max-height: unset;
		}
	}
</style>
