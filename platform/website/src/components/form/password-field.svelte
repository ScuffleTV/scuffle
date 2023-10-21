<script lang="ts">
	import Fa from "svelte-fa";
	import Field, { type FieldStatus } from "./field.svelte";
	import { faEye, faEyeSlash } from "@fortawesome/free-solid-svg-icons";

	export let label: string;
	export let autocomplete: string | undefined = undefined;
	export let required: boolean = false;
	export let placeholder: string | undefined = undefined;
	export let value: string = "";
	export let validate: ((v: string) => Promise<FieldStatus>) | undefined = undefined;
	export let status: FieldStatus | undefined = undefined;

	let type = "password";
	$: revealed = type === "text";

	function clickReveal() {
		if (revealed) {
			type = "password";
		} else {
			type = "text";
		}
	}
</script>

<Field {type} {label} {autocomplete} {required} {placeholder} bind:value {validate} bind:status>
	<button on:click={clickReveal} class="reveal-button" tabindex="-1" type="button">
		<Fa icon={revealed ? faEyeSlash : faEye} fw />
	</button>
</Field>

<style lang="scss">
	@import "../../assets/styles/variables.scss";

	.reveal-button {
		color: $textColorLight;
	}
</style>
