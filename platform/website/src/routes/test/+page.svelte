<script lang="ts">
    // Don't touch this when you don't know what you're doing
    // You've gotta do the 5Head math shit to understand most of this

    let overlay = true;
    let debug = false;

    // Only for debugging
    let xlDebug = 0;
    let xrDebug = 0;
    let ytDebug = 0;
    let ybDebug = 0;

    let result: string;

    let mouseStartX: number;
    let mouseStartY: number;
    let moveable: HTMLImageElement;
    let aspectRatio: number = 0;

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
        const xr = (x + width) + sX;
        const yb = (y + height) + sY;

        xlDebug = xl;
        xrDebug = xr;
        ytDebug = yt;
        ybDebug = yb;

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

    function center() {
        if (moveable) {   
            const width = moveable.naturalWidth / Math.min(moveable.naturalWidth, moveable.naturalHeight);
            const height = moveable.naturalHeight / Math.min(moveable.naturalWidth, moveable.naturalHeight);
            x = -(width / 2 - 0.5);
            y = -(height / 2 - 0.5);
        } else {
            x = 0;
            y = 0;
        }
    }

    const pickerSize = 40 * 16;

    // (0,0) is left/top, (1,1) is right/bottom
    let x = 0;
    let y = 0;

    const minScale = 1;
    const maxScale = 2;
    // between minScale and maxScale
    let scale = 1;

    $: applyLimits(), scale;

    let moving = false;

    function mouseDown(e: MouseEvent) {
        e.preventDefault();
        const xs = x * pickerSize;
        const ys = y * pickerSize;
        mouseStartX = e.clientX - xs;
        mouseStartY = e.clientY - ys;
        moving = true;
    }

    function mouseMove(e: MouseEvent) {
        if (!moving) return;

        e.preventDefault();

        // Update element position
        x = (e.clientX - mouseStartX) / pickerSize;
        y = (e.clientY - mouseStartY) / pickerSize;

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

    let files: FileList;

    function handleFiles(file: File) {
        const reader = new FileReader();

        reader.onload = (e) => {
            if (!e.target) return;
            moveable.src = e.target.result as string;
            calculateResult();
        };

        reader.readAsDataURL(file);
    }

    $: if (files && files[0]) {
        handleFiles(files[0]);
    }

    $: if (debug) {
        calculateResult(), scale, x, y;
    }

    function calculateResult() {
        if (!moveable) return;

        const canvas = document.createElement('canvas');
        const ctx = canvas.getContext('2d');

        if (!ctx) return;

        const width = moveable.naturalWidth / Math.min(moveable.naturalWidth, moveable.naturalHeight);
        const height = moveable.naturalHeight / Math.min(moveable.naturalWidth, moveable.naturalHeight);

        const sX = (width * (scale - 1)) / 2;
        const sY = (height * (scale - 1)) / 2;
        const xl = x - sX;
        const yt = y - sY;

        const rw = moveable.naturalWidth / (scale * width);
        const rh = moveable.naturalHeight / (scale * height);
        const xrl = (-xl) * rw;
        const yrt = (-yt) * rh;

        const r = Math.min(rw, rh);
        const ox = (r - rw) / 2;
        const oy = (r - rh) / 2;

        canvas.width = r;
        canvas.height = r;

        ctx.drawImage(moveable, xrl, yrt, rw, rh, ox, oy, rw, rh);
        
        result = canvas.toDataURL('image/png');
    }

    function save() {
        calculateResult();

        const link = document.createElement('a');
        link.download = 'image.png';
        link.href = result;
        link.click();
    }

    function updateAspectRatio() {
        aspectRatio = moveable?.naturalWidth / moveable?.naturalHeight;
        reset();
        applyLimits();
    }
</script>

<svelte:window on:mousemove={mouseMove} on:mouseup={mouseUp} />

<div class="content">
    <div class="images">
        <div class="wrapper" class:debug={debug} style="--size: {pickerSize}px">
            <img class="moveable" bind:this={moveable} draggable="false" on:mousedown={mouseDown} on:wheel={wheel} on:load={updateAspectRatio} class:wide={aspectRatio > 1} class:high={aspectRatio < 1} style="--scale: {scale}; --x: {x * pickerSize}px; --y: {y * pickerSize}px" alt="upload a file" />
            {#if overlay}
                <div class="mask"></div>
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
            {#if debug}
                <!-- Top left -->
                <div class="dot" style="--x: {xlDebug * pickerSize}px; --y: {ytDebug * pickerSize}px;">1</div>
                <!-- Top right -->
                <div class="dot" style="--x: {xrDebug * pickerSize}px; --y: {ytDebug * pickerSize}px;">2</div>
                <!-- Bottom left -->
                <div class="dot" style="--x: {xlDebug * pickerSize}px; --y: {ybDebug * pickerSize}px;">3</div>
                <!-- Bottom right -->
                <div class="dot" style="--x: {xrDebug * pickerSize}px; --y: {ybDebug * pickerSize}px;">4</div>
            {/if}
        </div>
        {#if debug}
            <img class="result" style="--size: {pickerSize}px" src={result} alt="result"/>
        {/if}
    </div>

    <div>
        <input type="file" bind:files={files}/>
        <button class="button primary" on:click={reset}>Reset</button>
        <button class="button primary" on:click={save}>Save</button>
        <button class="button primary" on:click={() => (overlay = !overlay)}>{overlay ? "Hide" : "Show"} Overlay</button>
        <button class="button primary" on:click={() => (debug = !debug)}>{debug ? "Hide" : "Show"} Debug</button>
    </div>
    <input type="range" min={minScale} max={maxScale} step="0.01" bind:value={scale} />

    <span>{scale}x</span>
    <span>x:{x}, y:{y}</span>

    <span>xl:{xlDebug}, xr:{xrDebug}</span>
    <span>yt:{ytDebug}</span>
    <span>yb:{ybDebug}</span>
</div>

<style lang="scss">
    @import "../../assets/styles/variables.scss";

    .content {
        display: flex;
        flex-direction: column;
        align-items: center;
        justify-content: center;
        width: 100vw;
        height: 100%;
    }

    .images {
        display: flex;
        justify-content: center;
        gap: 2rem;
        width: 100%;
        margin-bottom: 2rem;
        margin: 5rem;
    }

    .result {
        background-color: $bgColor2;
        width: var(--size);
        height: var(--size);
    }

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
        background: radial-gradient(transparent 70.5%, rgba(0, 0, 0, 0.75) 70.5%);
        pointer-events: none;

        &:after {
            content: "";
            position: absolute;
            top: 0;
            left: 0;
            bottom: 0;
            right: 0;
            border: 5px solid white;
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

    .dot {
        position: absolute;
        transform: translate(var(--x), var(--y)) translate(-50%, -50%);
        width: 0.5rem;
        height: 0.5rem;
        background-color: red;
        border-radius: 50%;
    }

    .moveable {
        position: absolute;
        cursor: move;
        transform-origin: center;
        transform: translate(var(--x), var(--y)) scale(var(--scale));
        min-width: 100%;
        min-height: 100%;

        &.wide {
            min-width: 100%;
            max-height: 100%;
        }

        &.high {
            min-height: 100%;
            max-width: 100%;
        }
    }
</style>
