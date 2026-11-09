// The PFD on this screen: open, and docked to the right half of the Map
// tab beside the map (split screen), over the whole window (full window) or
// floating.
export const pfdState = $state<{ open: boolean; split: boolean; full: boolean }>({ open: false, split: false, full: false });
