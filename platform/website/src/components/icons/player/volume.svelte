<script lang="ts">
	export let volume: number;
	export let size: number = 24;
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
	{#if volume > 0.0}
		<path
		id="wave1"
		d="M16.1838 8.99924C16.891 9.83775 17.2675 10.9057 17.2426 12.0023C17.2178 13.0989 16.7932 14.1486 16.0488 14.9542L17.1513 15.9742C18.1435 14.899 18.7086 13.4983 18.7404 12.0356C18.7721 10.5729 18.2684 9.14903 17.3238 8.03174L16.1838 8.99924Z"
		fill="currentColor"
		/>
	{/if}
	{#if volume > 0.5}
		<path
			id="wave2"
			d="M20.3728 6.05859L19.2253 7.02609C20.4042 8.42255 21.0326 10.2014 20.9926 12.0285C20.9525 13.8555 20.2467 15.6051 19.0078 16.9486L20.1103 17.9986C21.5981 16.3864 22.4458 14.2864 22.4941 12.0932C22.5424 9.90005 21.7881 7.76472 20.3728 6.08859V6.05859Z"
			fill="currentColor"
		/>
	{/if}
	<path id="speaker" d="M2 16V7.99998H7L13 1.99982H14V22H13L7 16H2Z" fill="currentColor" />
</svg>
