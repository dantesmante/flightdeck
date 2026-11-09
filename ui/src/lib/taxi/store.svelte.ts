// Taxi assist on the Map tab: the bar shown or not; `keep` while a
// clearance is entered, so hiding the bar keeps the route.
export const taxiState = $state<{ open: boolean; keep: boolean }>({ open: false, keep: false });
