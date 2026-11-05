// Whether the screen is phone-sized (the layout's own breakpoint).
const query = typeof window !== "undefined" ? window.matchMedia("(max-width: 640px)") : null;
export const screen = $state({ phone: query?.matches ?? false });
query?.addEventListener("change", (e) => (screen.phone = e.matches));
