<script lang="ts">
	import { onMount } from "svelte";

    let canvas: HTMLCanvasElement;
    let image: HTMLImageElement;

    function rescale() {
        const ctx = canvas.getContext('2d');
        ctx?.drawImage(image, 960/2-100, 540/2-100, 200, 200, 0, 0, 100, 100);
    }

    let moveable: HTMLElement;
    let mouseStartX: number;
    let mouseStartY: number;
    $: xLimit = (moveable?.parentElement?.offsetWidth || 0) - moveable?.offsetWidth;
    $: yLimit = (moveable?.parentElement?.offsetHeight || 0) - moveable?.offsetHeight;

    onMount(() => {
        moveable.addEventListener('mousedown', mouseDown);
        return () => {
            moveable.removeEventListener('mousedown', mouseDown);
            removeListeners();
        };
    });

    function mouseDown(e: MouseEvent) {
        e.preventDefault();
        mouseStartX = e.clientX - moveable.offsetLeft;
        mouseStartY = e.clientY - moveable.offsetTop;
        document.addEventListener('mousemove', mouseMove);
        document.addEventListener('mouseup', removeListeners);
    }

    function mouseMove(e: MouseEvent) {
        e.preventDefault();

        let newX = e.clientX - mouseStartX;
        let newY = e.clientY - mouseStartY;

        // Limit movement to parent element
        if (newX < 0) {
            newX = 0;
        } else if (newX > xLimit) {
            newX = xLimit;
        }
        if (newY < 0) {
            newY = 0;
        } else if (newY > yLimit) {
            newY = yLimit;
        }

        // Update element position
        moveable.style.left = newX + 'px';
        moveable.style.top = newY + 'px';
    }

    function removeListeners() {
        document.removeEventListener('mousemove', mouseMove);
        document.removeEventListener('mouseup', removeListeners);
    }
</script>

<!-- <canvas bind:this={canvas}></canvas> -->
<!-- <img bind:this={image} on:load={rescale} src="/banner.jpeg" /> -->

<div class="wrapper">
    <div class="moveable" bind:this={moveable}>
        Move me
    </div>
</div>

<style lang="scss">
    .wrapper {
        margin: 10rem;
        width: 50rem;
        height: 50rem;
        position: relative;
        background: #eee;
    }

    .moveable {
        width: 10rem;
        height: 10rem;
        background: #ccc;
        position: absolute;
        cursor: move;
    }
</style>
