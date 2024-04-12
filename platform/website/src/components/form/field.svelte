<script context="module" lang="ts">
	let fieldCounter = 0;

	export enum FieldStatusType {
		None = "",
		Success = "success",
		Error = "error",
		Warning = "warning",
		Loading = "loading",
	}

	export type FieldStatus = { type: FieldStatusType; message?: string };

	let resetFns: Map<number, () => void> = new Map();

	export function resetAllFields() {
		for (let reset of Object.values(resetFns)) {
			reset();
		}
	}
</script>

<script lang="ts">
	import Fa from "svelte-fa";
	import { faCheck, faTriangleExclamation, faXmark } from "@fortawesome/free-solid-svg-icons";
	import Spinner from "../spinner.svelte";
	import { onMount } from "svelte";

	export let type: string = "text";
	export let label: string | undefined = undefined;
	export let autocomplete: string | undefined = undefined;
	export let required: boolean = false;
	export let disabled: boolean = false;
	export let placeholder: string | undefined = undefined;
	export let value: string = "";
	export let validate: (v: string) => Promise<FieldStatus> = () =>
		new Promise((resolve) => resolve({ type: FieldStatusType.Success }));
	export let status: FieldStatus = { type: FieldStatusType.None };

	let initialValue = value;
	let touched = false;

	let id = fieldCounter++;

	onMount(() => {
		resetFns.set(id, () => {
			touched = false;
			value = "";
			status = { type: FieldStatusType.None };
		});
		return () => {
			resetFns.delete(id);
		};
	});

	$: if (value !== initialValue) {
		touched = true;
	}

	$: if (touched) {
		status = { type: FieldStatusType.Loading };
		validate(value)
			.then((s) => (status = s))
			.catch((e) => {
				// Ignore rejected promises with no error
				if (e) {
					console.error(e);
				}
			});
	}
</script>

<div class="field-outer {status.type}">
	{#if label}
		<label for="field-{id}">{label}</label>
	{/if}
	<div class="field">
		<!-- https://stackoverflow.com/a/75298645/10772729 -->
		<input
			id="field-{id}"
			{placeholder}
			{autocomplete}
			{required}
			{disabled}
			bind:value
			{...{ type }}
		/>
		<div class="input-inner">
			<slot />
			{#if status.type === FieldStatusType.Loading}
				<div class="input-status">
					<Spinner />
				</div>
			{:else if status.type === FieldStatusType.Success}
				<div class="input-status">
					<Fa icon={faCheck} />
				</div>
			{:else if status.type === FieldStatusType.Error}
				<div class="input-status">
					<Fa icon={faXmark} />
				</div>
			{:else if status.type === FieldStatusType.Warning}
				<div class="input-status">
					<Fa icon={faTriangleExclamation} />
				</div>
			{/if}
		</div>
	</div>
	<!-- We don't conditionally render here because we want smooth animations, so we just toggle visibility with some "hidden" text -->
	<span id="field-{id}-message" class="message" class:visible={status.message}
		>{status.message}</span
	>
</div>

<style lang="scss">
	@import "../../assets/styles/variables.scss";

	.field-outer {
		display: flex;
		flex-direction: column;
		// margin-bottom: 1rem;
		text-align: left;

		&.success {
			input {
				border-color: $successColor !important;
				background-color: black;
			}

			.message {
				color: $successColorDark;
			}

			.input-status {
				color: $successColor;
			}
		}

		&.error {
			input {
				border-color: $errorColor !important;
				background-color: black;
			}

			.message {
				color: $errorColorDark;
			}

			.input-status {
				color: $errorColor;
			}
		}

		&.warning {
			input {
				border-color: $warningColor !important;
				background-color: black;
			}

			.message {
				color: $warningColorDark;
			}

			.input-status {
				color: $warningColor;
			}
		}

		&.loading {
			input {
				border-color: $loadingColor !important;
				background-color: black;
			}

			.message {
				color: $loadingColor;
			}

			.input-status {
				color: $loadingColor;
			}
		}
	}

	label {
		margin-bottom: 0.5rem;
		color: $textColorLighter;
	}

	.field {
		position: relative;

		input {
			padding: 0.5rem;
			border-radius: 0.25rem;
			outline: 1px solid $borderColor;
			background-color: $bgColor2;
			border: 2px solid transparent;
			box-shadow: 0px 4px 12px 4px rgba(0, 0, 0, 0.15);
			transition: border-color 0.25s ease-in-out;
			width: 100%;

			color: $textColor;

			&:hover,
			&:focus-visible {
				border-color: $borderColor;
			}

			&:focus {
				background-color: black;
				border-color: $primaryColor;
				box-shadow: 0px 4px 4px rgba(0, 0, 0, 0.25);
			}
		}
	}

	.message {
		margin-top: 0.1rem;
		font-size: 0.9rem;
		max-height: 0;
		color: $textColorLight;
		overflow: hidden;
		transition: max-height 0.25s ease;
		visibility: hidden;

		&.visible {
			visibility: visible;
			max-height: 1rem;
		}
	}

	.input-status {
		font-size: 1.25em;
		padding: 0.25rem 0;
		margin-top: 0.1em;
		margin-left: 0.25rem;
	}

	.input-inner {
		position: absolute;
		top: 0;
		bottom: 0;
		right: 0;
		display: flex;
		font-size: 1rem;
		padding: 0 0.5rem;
	}
</style>
