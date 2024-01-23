<script lang="ts">
    let mouseStartX: number;
    let mouseStartY: number;
    let moveable: HTMLImageElement;

    function applyLimits() {
        const parent = moveable?.parentElement;
        if (!parent) return;

        const width = moveable.offsetWidth;
        const height = moveable.offsetHeight;

        const xMinLimit = parent.offsetWidth - width * scale;
        const yMinLimit = parent.offsetHeight - height * scale;
        const xMaxLimit = 0;
        const yMaxLimit = 0;

        const magicValueX = (width * (scale - 1)) / 2;
        const magicValueY = (height * (scale - 1)) / 2;

        const actualX = x - magicValueX;
        const actualY = y - magicValueY;

        // Limit movement to parent element
        if (actualX > xMaxLimit) {
            x = xMaxLimit + magicValueX;
        } else if (actualX < xMinLimit) {
            x = xMinLimit + magicValueX;
        }
        if (actualY > yMaxLimit) {
            y = yMaxLimit + magicValueY;
        } else if (actualY < yMinLimit) {
            y = yMinLimit + magicValueY;
        }
    }

    function reset() {
        // Reset scale
        scale = 1;
        center();
    }

    function center() {
        const parent = moveable?.parentElement;
        // Center image
        if (!parent) return;
        x = (parent.offsetWidth / 2) - (moveable.offsetWidth * scale / 2);
        y = (parent.offsetHeight / 2) - (moveable.offsetHeight * scale / 2);
    }

    // between 0 and 1
    let x = 0;
    let y = 0;

    const minScale = 1;
    const maxScale = 4;
    // between minScale and maxScale
    let scale = 1;

    $: applyLimits(), scale;

    let moving = false;

    function mouseDown(e: MouseEvent) {
        e.preventDefault();
        mouseStartX = e.clientX - x;
        mouseStartY = e.clientY - y;
        moving = true;
    }

    function mouseMove(e: MouseEvent) {
        if (!moving) return;

        e.preventDefault();

        // Update element position
        x = e.clientX - mouseStartX;
        y = e.clientY - mouseStartY;

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
        };

        reader.readAsDataURL(file);
    }

    $: if (files && files[0]) {
        handleFiles(files[0]);
    }

    function save() {
        const canvas = document.createElement('canvas');
        const ctx = canvas.getContext('2d');

        if (!ctx) return;

        const parent = moveable?.parentElement;
        if (!parent) return;

        const resultDimension = Math.max(moveable.naturalWidth / scale, moveable.naturalHeight / scale);

        canvas.width = resultDimension;
        canvas.height = resultDimension;

        let xCorrection = 0;
        let yCorrection = 0;
        // if (moveable.naturalWidth > moveable.naturalHeight) {
        //     yCorrection = (moveable.naturalWidth - moveable.naturalHeight) / 2;
        // } else if (moveable.naturalHeight > moveable.naturalWidth) {
        //     xCorrection = (moveable.naturalHeight - moveable.naturalWidth) / 2;
        // }

        const magicValueX = (moveable.offsetWidth * (scale - 1)) / 2;
        const magicValueY = (moveable.offsetHeight * (scale - 1)) / 2;
        const actualX = ((x - magicValueX) / (moveable.offsetWidth * scale)) * moveable.naturalWidth + xCorrection;
        const actualY = ((y - magicValueY) / (moveable.offsetHeight * scale)) * moveable.naturalHeight + yCorrection;

        ctx.drawImage(moveable, -actualX, -actualY, resultDimension, resultDimension, 0, 0, resultDimension, resultDimension);
        
        const data = canvas.toDataURL('image/png');

        const link = document.createElement('a');
        link.download = 'image.png';
        link.href = data;
        link.click();
    }
</script>

<svelte:window on:mousemove={mouseMove} on:mouseup={mouseUp} />

<div class="content">
    <div class="wrapper">
        <img class="moveable" bind:this={moveable} draggable="false" on:mousedown={mouseDown} on:wheel={wheel} style="--scale: {scale}; --x: {x * moveable.offsetWidth}px; --y: {y * moveable.offsetHeight}px" alt="upload a file" />
        <div class="mask"></div>
        <div class="grid">
            <div></div>
            <div></div>
            <div></div>
            <div></div>
            <div></div>
            <div></div>
            <div></div>
            <div></div>
            <div></div>
        </div>
    </div>

    <input type="file" bind:files={files}/>

    <button class="button primary" on:click={reset}>Reset</button>

    <input type="range" min={minScale} max={maxScale} step="0.01" bind:value={scale} />

    <span>{scale}x</span>
    <span>x:{x}, y:{y}</span>
    <span>natural size: {moveable?.naturalWidth}x{moveable?.naturalHeight}</span>

    <button class="button primary" on:click={save}>Save</button>
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

    .wrapper {
        margin: 5rem;
        width: 40rem;
        height: 40rem;
        position: relative;
        background-color: $bgColor;

        overflow: hidden;
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
            border: 1px solid white;
        }
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
