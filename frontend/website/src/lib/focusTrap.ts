// Taken from https://github.com/skeletonlabs/skeleton/blob/384982bc4ba34d01f9b618c6064f712585ae0511/src/lib/actions/FocusTrap/focusTrap.ts
// https://www.skeleton.dev/actions/focus-trap
// MIT

// Essentially, this focus-trap causes the first and last focusable elements to wrap around when tabbing.
// This is useful for modals, where you don't want the user to tab out of the modal.
// It also auto-focuses the first focusable element when the modal is opened.

// Action: Focus Trap

export function focusTrap(node: HTMLElement, enabled: boolean) {
	const elemWhitelist =
		'a[href], button, input, textarea, select, details, [tabindex]:not([tabindex="-1"])';
	let elemFirst: HTMLElement;
	let elemLast: HTMLElement;

	// When the first element is selected, shift+tab pressed, jump to the last selectable item.
	function onFirstElemKeydown(e: KeyboardEvent): void {
		if (e.shiftKey && e.code === "Tab") {
			e.preventDefault();
			elemLast.focus();
		}
	}

	// When the last item selected, tab pressed, jump to the first selectable item.
	function onLastElemKeydown(e: KeyboardEvent): void {
		if (!e.shiftKey && e.code === "Tab") {
			e.preventDefault();
			elemFirst.focus();
		}
	}

	const onInit = () => {
		if (enabled === false) return;
		// Gather all focusable elements
		const focusableElems: HTMLElement[] = Array.from(node.querySelectorAll(elemWhitelist));
		if (focusableElems.length) {
			// Set first/last focusable elements
			elemFirst = focusableElems[0];
			elemLast = focusableElems[focusableElems.length - 1];
			// Auto-focus first focusable element
			elemFirst.focus();
			// Listen for keydown on first & last element
			elemFirst.addEventListener("keydown", onFirstElemKeydown);
			elemLast.addEventListener("keydown", onLastElemKeydown);
		}
	};
	onInit();

	function onDestory(): void {
		if (elemFirst) elemFirst.removeEventListener("keydown", onFirstElemKeydown);
		if (elemLast) elemLast.removeEventListener("keydown", onLastElemKeydown);
	}

	// Lifecycle
	return {
		update(newArgs: boolean) {
			enabled = newArgs;
			newArgs ? onInit() : onDestory();
		},
		destroy() {
			onDestory();
		},
	};
}
