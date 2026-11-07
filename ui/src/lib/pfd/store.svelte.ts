// The PFD on this screen: open, and docked to the right half of the Map
// tab beside the map (split screen) or floating.
export const pfdState = $state<{ open: boolean; split: boolean }>({ open: false, split: false });
