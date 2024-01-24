<script lang="ts">
    // Don't touch this when you don't know what you're doing
    // You've gotta do the 5Head math shit to understand most of this

    let overlay = true;
    let debug = false;

    let result: string;

    let mouseStartX: number;
    let mouseStartY: number;
    let moveable: HTMLImageElement;

    function applyLimits() {
        const minLimit = 1 - scale;
        const maxLimit = 0;

        const s = (scale - 1) / 2;
        const xl = x - s;
        const yt = y - s;

        // Limit movement to parent element
        if (xl > maxLimit) {
            x = maxLimit + s;
        } else if (xl < minLimit) {
            x = minLimit + s;
        }
        if (yt > maxLimit) {
            y = maxLimit + s;
        } else if (yt < minLimit) {
            y = minLimit + s;
        }
    }

    function reset() {
        // Reset scale
        scale = 1;
        // Reset position
        x = 0;
        y = 0;
    }

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
        const xs = x * moveable.offsetWidth;
        const ys = y * moveable.offsetHeight;
        mouseStartX = e.clientX - xs;
        mouseStartY = e.clientY - ys;
        moving = true;
    }

    function mouseMove(e: MouseEvent) {
        if (!moving) return;

        e.preventDefault();

        // Update element position
        x = (e.clientX - mouseStartX) / moveable.offsetWidth;
        y = (e.clientY - mouseStartY) / moveable.offsetHeight;

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

    $: calculateResult(), scale, x, y;

    function calculateResult() {
        if (!moveable) return;

        const canvas = document.createElement('canvas');
        const ctx = canvas.getContext('2d');

        if (!ctx) return;

        const xl = x - (scale - 1) / 2;
        const xr = (x+1) + (scale - 1) / 2;
        const yt = y - (scale - 1) / 2;
        const yb = (y+1) + (scale - 1) / 2;
        console.log("xl,yt,xr,yb", xl, yt, xr, yb);

        const rw = moveable.naturalWidth / scale;
        const rh = moveable.naturalHeight / scale;
        const xrl = (-xl) * rw;
        const yrt = (-yt) * rh;

        const r = Math.min(rw, rh);
        const ox = (r - rw) / 2;
        const oy = (r - rh) / 2;

        canvas.width = r;
        canvas.height = r;

        console.log(xrl, yrt, rw, rh, ox, oy, rw, rh);
        ctx.drawImage(moveable, xrl, yrt, rw, rh, ox, oy, rw, rh);
        
        result = canvas.toDataURL('image/png');
    }
</script>

<svelte:window on:mousemove={mouseMove} on:mouseup={mouseUp} />

<div class="content">
    <div class="images">
        <div class="wrapper" class:debug={debug}>
            <img class="moveable" bind:this={moveable} draggable="false" on:mousedown={mouseDown} on:wheel={wheel} style="--scale: {scale}; --x: {x * (moveable?.offsetWidth || 0)}px; --y: {y * (moveable?.offsetHeight || 0)}px" alt="upload a file" />
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
                <div class="dot" style="--x: {(x - (scale - 1) / 2) * (moveable?.offsetWidth || 0)}px; --y: {(y - (scale - 1) / 2) * (moveable?.offsetHeight || 0)}px;"></div>
                <!-- Top right -->
                <div class="dot" style="--x: {((x+1) + (scale - 1) / 2) * (moveable?.offsetWidth || 0)}px; --y: {(y - (scale - 1) / 2) * (moveable?.offsetHeight || 0)}px;"></div>
                <!-- Bottom left -->
                <div class="dot" style="--x: {(x - (scale - 1) / 2) * (moveable?.offsetWidth || 0)}px; --y: {((y+1) + (scale - 1) / 2) * (moveable?.offsetHeight || 0)}px;"></div>
                <!-- Bottom right -->
                <div class="dot" style="--x: {((x+1) + (scale - 1) / 2) * (moveable?.offsetWidth || 0)}px; --y: {((y+1) + (scale - 1) / 2) * (moveable?.offsetHeight || 0)}px;"></div>
            {/if}
        </div>
        <img class="result" src={result} alt="result"/>
    </div>

    <div>
        <input type="file" bind:files={files}/>
        <button class="button primary" on:click={reset}>Reset</button>
        <button class="button primary" on:click={() => (overlay = !overlay)}>{overlay ? "Hide" : "Show"} Overlay</button>
        <button class="button primary" on:click={() => (debug = !debug)}>{debug ? "Hide" : "Show"} Debug</button>
    </div>
    <input type="range" min={minScale} max={maxScale} step="0.01" bind:value={scale} />

    <span>{scale}x</span>
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
        width: 40rem;
        height: 40rem;
    }

    .wrapper {
        width: 40rem;
        height: 40rem;
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
        transform: translate(var(--x), var(--y)) scale(var(--scale));
        width: 100%;
        height: 100%;
        object-fit: contain;
    }
</style>
