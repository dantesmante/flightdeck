import type { Phase } from "./api";

export const PHASES: { id: Phase; label: string }[] = [
  { id: "preflight", label: "Preflight" },
  { id: "taxi_out", label: "Taxi out" },
  { id: "takeoff_roll", label: "Takeoff roll" },
  { id: "airborne", label: "Airborne" },
  { id: "landing_rollout", label: "Landing rollout" },
  { id: "taxi_in", label: "Taxi in" },
  { id: "arrived", label: "Arrived" },
];

const n0 = new Intl.NumberFormat("en-US", { maximumFractionDigits: 0 });
const n1 = new Intl.NumberFormat("en-US", { maximumFractionDigits: 1 });

export const int = (v: number | null | undefined) => (v == null ? "–" : n0.format(Math.round(v)));
export const dec = (v: number | null | undefined) => (v == null ? "–" : n1.format(v));

export function signed(v: number | null | undefined): string {
  if (v == null) return "–";
  const r = Math.round(v);
  return r > 0 ? `+${n0.format(r)}` : n0.format(r);
}

export function hhmm(minutes: number): string {
  const m = Math.max(0, Math.round(minutes));
  return `${Math.floor(m / 60)}:${String(m % 60).padStart(2, "0")}`;
}

export function hoursMinutes(minutes: number): string {
  const m = Math.max(0, Math.round(minutes));
  const h = Math.floor(m / 60);
  return h ? `${h} h ${String(m % 60).padStart(2, "0")} min` : `${m} min`;
}

/** Logbook times are UTC. */
export function zulu(ms: number | null | undefined): string {
  if (ms == null) return "–";
  const d = new Date(ms);
  return `${String(d.getUTCHours()).padStart(2, "0")}${String(d.getUTCMinutes()).padStart(2, "0")}Z`;
}

const months = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
export function day(ms: number): string {
  const d = new Date(ms);
  return `${d.getUTCDate()} ${months[d.getUTCMonth()]} ${d.getUTCFullYear()}`;
}

export function heading(v: number | null | undefined): string {
  if (v == null) return "–";
  const h = Math.round(((v % 360) + 360) % 360) || 360;
  return String(h).padStart(3, "0");
}

/** Color for a landing-rate grade. */
export function gradeTone(grade: string | null | undefined): "good" | "warn" | "bad" | "dim" {
  switch (grade) {
    case "Butter":
    case "Smooth":
      return "good";
    case "Firm":
      return "warn";
    case "Hard":
    case "Very hard":
      return "bad";
    default:
      return "dim";
  }
}
