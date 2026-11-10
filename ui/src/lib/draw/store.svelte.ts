// The pen shared by the scratch pad and map markup: colour, size, eraser.
export const PEN_COLORS = ["#f2f4f7", "#f0b23a", "#5cc8e8", "#ff5a5f", "#5fd38d", "#b98cff"];
export const pen = $state<{ color: string; size: number; erasing: boolean }>({ color: PEN_COLORS[1], size: 3, erasing: false });
// Map markup (Tools → Markup): drawing on the map, or not.
export const markupState = $state<{ open: boolean }>({ open: false });
