// The flight profile under the map on the Map tab: open or not, and its
// height in pixels (null: half the window), remembered on this computer.
function savedHeight(): number | null {
  try {
    const v = Number(localStorage.getItem("fd.profileH"));
    return Number.isFinite(v) && v > 0 ? v : null;
  } catch {
    return null;
  }
}
export const profileState = $state<{ open: boolean; hover: [number, number] | null; h: number | null }>({ open: false, hover: null, h: savedHeight() });
export function saveProfileHeight(h: number) {
  try {
    localStorage.setItem("fd.profileH", String(Math.round(h)));
  } catch {
    /* not kept: fine */
  }
}
/** The smallest the profile gets while still readable. */
export const PROFILE_MIN_H = 240;
