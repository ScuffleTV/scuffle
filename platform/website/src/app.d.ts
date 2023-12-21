// See https://kit.svelte.dev/docs/types#app
// for information about these interfaces
declare global {
	namespace App {
		// interface Error {}
		// interface Locals {}
		// interface PageData {}
		// interface Platform {}
	}

	interface Window {
		turnstile: {
			render: (element: string | HTMLElement, options: TurnstileOptions) => string;
			remove: (widgetId: string) => void;
			reset: (widgetId: string) => void;
		};
	}
}

export {};
