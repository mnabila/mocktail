export function reveal(node: HTMLElement) {
	const observer = new IntersectionObserver(
		(entries) => {
			for (const entry of entries) {
				if (!entry.isIntersecting) continue;
				entry.target.classList.add('is-in');
				observer.unobserve(entry.target);
			}
		},
		{ threshold: 0.12, rootMargin: '0px 0px -6% 0px' }
	);

	observer.observe(node);

	return {
		destroy() {
			observer.disconnect();
		}
	};
}
