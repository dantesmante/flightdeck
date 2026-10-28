// Planespotters photos by registration, looked up once per session (the
// app side keeps them for up to 24 hours, as Planespotters allows).
import { api, type SpotterPhoto } from "../api";

export const spotters = $state<Record<string, SpotterPhoto | null>>({});
const pending = new Set<string>();

export async function loadSpotter(registration: string) {
  const reg = registration.trim().toUpperCase();
  if (!reg || reg in spotters || pending.has(reg)) return;
  pending.add(reg);
  try {
    spotters[reg] = (await api.spotterPhoto(reg)) ?? null;
  } catch {
    spotters[reg] = null;
  } finally {
    pending.delete(reg);
  }
}
