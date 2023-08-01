<script context="module" lang="ts">
	export type StatusType = "success" | "error" | "warning" | "loading" | "";

	//
	// Field behavior
	// 	Inside functions on the fields we cannot use `this` to refer to the field object.
	// 	This is because the svelte compiler does not know that `this` refers to the field object,
	//  Thus it does not detect changes to the field object and does not update the UI.
	//  Even though we actually do mutate the same object, svelte does not detect it.
	//  You can learn more about how svelte works here: https://svelte.dev/tutorial/updating-arrays-and-objects
	//

	export interface LoginFieldOptions<T = Record<string, never>> {
		id?: string;
		type?: string;
		label?: string;
		placeholder?: string;
		value?: string;
		message?: string;
		status?: StatusType;
		touched?: boolean;
		extra?: T;
		required?: boolean;
		autoComplete?: string;
		validate?: (this: unknown, value: string) => boolean;
		valid?: (this: unknown) => boolean;
		update?: (this: unknown, value: string) => void;
		reload?: (this: unknown) => void;
	}

	export interface LoginFieldType<T = Record<string, never>> extends LoginFieldOptions<T> {
		id: string;
		type: string;
		label: string;
		required: boolean;
		autoComplete: string;
		placeholder: string;
		value: string;
		message: string;
		status: StatusType;
		touched: boolean;
		extra: T;
		// Defining `this: unknown` causes typescript to get mad when we use `this` inside the function.
		// This is because `this` is not actually `unknown` but is the field object. However we should discourage the use of `this` inside the functions,
		// Since it creates undefined behavior as explained above.
		validate: (this: unknown, value: string) => boolean;
		valid: (this: unknown) => boolean;
		update: (this: unknown, value: string) => void;
		reload: (this: unknown) => void;
	}

	export function newField<T = Record<string, never>>(
		props: LoginFieldOptions<T>,
	): LoginFieldType<T> {
		return {
			id: props.id || "",
			type: props.type || "text",
			label: props.label || "",
			autoComplete: props.autoComplete || "off",
			required: props.required || false,
			placeholder: props.placeholder || "",
			value: props.value || "",
			message: props.message || "",
			status: props.status || "",
			touched: props.touched || false,
			extra: props.extra || ({} as T),
			validate: props.validate || (() => true),
			valid: props.valid || (() => true),
			// eslint-disable-next-line @typescript-eslint/no-empty-function
			update: props.update || (() => {}),
			// eslint-disable-next-line @typescript-eslint/no-empty-function
			reload: props.reload || (() => {}),
		};
	}
</script>

<script lang="ts">
	import Fa from "svelte-fa";
	import {
		faCircleNotch,
		faCheck,
		faTriangleExclamation,
		faXmark,
		faEyeSlash,
		faEye,
	} from "@fortawesome/free-solid-svg-icons";

	// This is safe becasuse we never access the extra field to which the type is templated.
	// eslint-disable-next-line @typescript-eslint/no-explicit-any
	export let field: LoginFieldType<any>;

	let oldValue = "";
	let value = field.value;
	let passwordVisible = false;
	let inputEl: HTMLInputElement;

	$: {
		// This branch is true when the user enters data into the field.
		if (oldValue !== value) {
			let invalidFirstTouch = false;

			if (!field.validate(value)) {
				value = oldValue;
				invalidFirstTouch = !field.touched;
				field.touched = true;
			}

			if (value.length > 0) {
				field.touched = true;
			}

			if ((field.touched && value !== oldValue) || invalidFirstTouch) {
				field.update(value);
				oldValue = value;
			}
		} else if (field.value !== value) {
			// This branch is true when the field is updated from the outside.
			value = field.value;
		}
	}

	// Svelte does not allow 2 way binding on input type, so we have to do it manually.
	function useType(node: HTMLInputElement) {
		node.type = field.type === "password" ? (passwordVisible ? "text" : "password") : field.type;
	}

	// When the user clicks the password visibility toggle, we toggle the password visibility.
	// Only called when type is password.
	function togglePasswordVisible() {
		passwordVisible = !passwordVisible;
		useType(inputEl);
	}
</script>

<div class={"form-group " + (field.touched ? field.status : "")}>
	<label for={field.id}>{field.label}</label>
	<!-- This wrapper allows for relative CSS positioning -->
	<div class="input-group">
		<input
			id={field.id}
			placeholder={field.placeholder}
			autocomplete={field.autoComplete}
			required={field.required}
			bind:value
			use:useType
			bind:this={inputEl}
		/>
		<!-- This wrapper is absolute and relative to the input field and it makes it easy to do CSS positioning -->
		<div class="field-inliner">
			<!-- Only render the password visibility toggle if the type is password -->
			{#if field.type === "password"}
				<button
					class="password-visible"
					on:click={togglePasswordVisible}
					tabindex="-1"
					type="button"
				>
					{#if passwordVisible}
						<Fa icon={faEyeSlash} />
					{:else}
						<Fa icon={faEye} />
					{/if}
				</button>
			{/if}
			<!-- Only render the status icon if the field has been touched, otherwise on first load everything will be of type "error" -->
			<!-- Touched only happens when the value has been modified once -->
			{#if field.touched}
				{#if field.status === "loading"}
					<div class="input-status">
						<Fa icon={faCircleNotch} spin />
					</div>
				{:else if field.status === "success"}
					<div class="input-status">
						<Fa icon={faCheck} />
					</div>
				{:else if field.status === "error"}
					<div class="input-status">
						<Fa icon={faXmark} />
					</div>
				{:else if field.status === "warning"}
					<div class="input-status">
						<Fa icon={faTriangleExclamation} />
					</div>
				{/if}
			{/if}
		</div>
	</div>
	<!-- We don't conditionally render here because we want smooth animations, so we just toggle visibility with some "hidden" text -->
	<span class="message" class:visible={!!field.message && field.touched}
		>{field.touched && (field.message || "hidden")}</span
	>
</div>

<style lang="scss">
	@import "../../assets/styles/variables.scss";

	.form-group {
		font-family: inherit;
		position: relative;
		display: flex;
		flex-direction: column;
		margin-bottom: 1rem;
		text-align: left;

		label {
			margin-bottom: 0.5rem;
			opacity: 85%;
		}

		&.success {
			input {
				border-color: $successColor !important;
				background-color: black;
			}

			.message {
				color: #3e9c3e;
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
				color: #a03e3e;
			}

			.input-status {
				color: $errorColor;
			}
		}

		&.warning {
			input {
				border-color: #ff8c00 !important;
				background-color: black;
			}

			.message {
				color: #ad7033;
			}

			.input-status {
				color: #ff8c00;
			}
		}

		&.loading {
			input {
				border-color: #b7e779 !important;
				background-color: black;
			}

			.message {
				color: #b7e779;
			}

			.input-status {
				color: #b7e779;
			}
		}

		input {
			font-family: inherit;
			padding: 0.5rem;
			border-radius: 0.25rem;
			outline: 1px solid #2c2c2c;
			background-color: #161818;
			border: 2px solid transparent;
			box-shadow: 0px 4px 12px 4px #00000026;
			transition: border-color 0.25s ease-in-out;
			width: 100%;

			color: $textColor;

			&:hover {
				border-color: #393939;
			}

			&:focus {
				background-color: black;
				border-color: $primaryColor;
				box-shadow: 0px 4px 4px #0000003f;
			}
		}

		.message {
			margin-top: 0.1rem;
			font-size: 0.9rem;
			color: #9c9c9c;
			max-height: 0;
			overflow: hidden;
			transition: max-height 0.25s ease;
			visibility: hidden;
			&.visible {
				visibility: visible;
				max-height: 1rem;
			}
		}

		.input-group {
			position: relative;
			width: 100%;
		}

		.field-inliner {
			position: absolute;
			right: 0;
			top: 0;
			z-index: 1;
			display: flex;
			font-size: 1rem;
			padding: 0 0.5rem;
		}

		.input-status {
			font-size: 1.25em;
			padding: 0.25rem 0;
			margin-top: 0.1em;
			margin-left: 0.25rem;
		}

		.password-visible {
			background-color: transparent;
			padding: 0.5rem 0;
			color: rgba(255, 255, 255, 0.35);
			border: 0;
			z-index: 1;
			font: inherit;
			margin: 0;
			width: 1.5rem;
		}
	}
</style>
