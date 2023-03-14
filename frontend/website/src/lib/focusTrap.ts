const focussableElements = "a, button, input";

export function focusTrap(node: HTMLElement, enabled: boolean) {
	function getTree(): HTMLElement[] {
		let elements: HTMLElement[] = Array.from(node.querySelectorAll(focussableElements));
		elements = elements.filter((element) => {
			return (
				element.tabIndex !== -1 &&
				element.getAttribute("disabled") === null &&
				element.getAttribute("type") !== "hidden"
			);
		});

		return elements;
	}

	let isEnabled = false;

	function keydown(event: KeyboardEvent) {
		if (!isEnabled) {
			return;
		}

		if (event.key !== "Tab") {
			return;
		}

		event.preventDefault();

		const tree = getTree();
		const current = document.activeElement as HTMLElement;

		const modify = event.shiftKey ? -1 : 1;
		const currentIndex = tree.indexOf(current);

		// if we were not in the tree, if they press shift we should focus the last element
		// otherwise we should focus the first element
		// -1 + 1 = 0 (which is the first element)
		// -1 - 1 = -2 (which is the last element)

		const nextIndex = currentIndex + modify;
		if (nextIndex < 0) {
			tree[tree.length - 1].focus();
		} else if (nextIndex >= tree.length) {
			tree[0].focus();
		} else {
			tree[nextIndex].focus();
		}
	}

	function init() {
		window.addEventListener("keydown", keydown);
	}

	function update(enabled: boolean) {
		isEnabled = enabled;
	}

	function destroy() {
		isEnabled = false;
		window.removeEventListener("keydown", keydown);
	}

	init();
	update(enabled);

	return {
		update,
		destroy,
	};
}
