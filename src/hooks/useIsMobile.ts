import { useSyncExternalStore } from "react";

/** Viewport width (px) below which the compact phone layout is used. Matches Tailwind's `md` breakpoint. */
export const MOBILE_BREAKPOINT = 768;

const MOBILE_QUERY = `(max-width: ${MOBILE_BREAKPOINT - 1}px)`;

function subscribe(onChange: () => void) {
	const mq = window.matchMedia(MOBILE_QUERY);
	mq.addEventListener("change", onChange);
	return () => mq.removeEventListener("change", onChange);
}

function getSnapshot() {
	return window.matchMedia(MOBILE_QUERY).matches;
}

/** True when the viewport is phone-sized (e.g. a portrait Android device). */
export function useIsMobile() {
	return useSyncExternalStore(subscribe, getSnapshot, () => false);
}
