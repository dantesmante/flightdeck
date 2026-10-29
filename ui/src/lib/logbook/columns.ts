// Logbook column layouts. Each column sits on the left or right page of a
// spread; times print as decimal hours (FAA) or hours:minutes (EASA).

import type { LogRow } from "../api";

export type Layout = "faa" | "easa";

export interface Col {
  key: string;
  label: string;
  /** Printed above a group of columns, e.g. "Landings". */
  group?: string;
  width: number;
  side: "L" | "R";
  /** Text or minutes/count. Numeric columns get page totals. */
  kind: "text" | "minutes" | "count";
  value: (r: LogRow) => string | number;
  /** Wraps or shrinks long text. */
  flex?: boolean;
}

const months = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
const date = (r: LogRow) => {
  const d = new Date(r.date_ms);
  return `${d.getUTCDate()} ${months[d.getUTCMonth()]} ${String(d.getUTCFullYear()).slice(2)}`;
};
const hhmmZ = (ms: number | null) => {
  if (ms == null) return "";
  const d = new Date(ms);
  return `${String(d.getUTCHours()).padStart(2, "0")}${String(d.getUTCMinutes()).padStart(2, "0")}`;
};
// Category and class decide the columns: SEL and MEL (land) for the FAA
// layout; single- or multi-engine for EASA.
const is = (r: LogRow, ...c: string[]) => c.includes(r.category_class);

export const LAYOUTS: Record<Layout, Col[]> = {
  faa: [
    { key: "date", label: "Date", width: 66, side: "L", kind: "text", value: date },
    { key: "type", label: "Aircraft type", width: 64, side: "L", kind: "text", value: (r) => r.aircraft_type },
    { key: "ident", label: "Aircraft ident", width: 70, side: "L", kind: "text", value: (r) => r.ident },
    { key: "from", label: "From", group: "Route of flight", width: 50, side: "L", kind: "text", value: (r) => r.from_icao },
    { key: "to", label: "To", group: "Route of flight", width: 50, side: "L", kind: "text", value: (r) => r.to_icao },
    { key: "remarks", label: "Remarks, procedures, maneuvers", width: 0, side: "L", kind: "text", value: (r) => r.remarks, flex: true },

    { key: "app", label: "Instr. app.", width: 34, side: "R", kind: "count", value: (r) => r.approaches },
    { key: "ldg_day", label: "Day", group: "Landings", width: 36, side: "R", kind: "count", value: (r) => r.day_landings },
    { key: "ldg_night", label: "Night", group: "Landings", width: 36, side: "R", kind: "count", value: (r) => r.night_landings },
    { key: "sel", label: "SEL", group: "Category and class", width: 46, side: "R", kind: "minutes", value: (r) => (is(r, "SEL") ? r.total_min : 0) },
    { key: "mel", label: "MEL", group: "Category and class", width: 46, side: "R", kind: "minutes", value: (r) => (is(r, "MEL") ? r.total_min : 0) },
    { key: "xc", label: "Cross-country", width: 48, side: "R", kind: "minutes", value: (r) => r.xc_min },
    { key: "night", label: "Night", width: 44, side: "R", kind: "minutes", value: (r) => r.night_min },
    { key: "act", label: "Actual", group: "Instrument", width: 44, side: "R", kind: "minutes", value: (r) => r.act_inst_min },
    { key: "sim", label: "Hood / sim.", group: "Instrument", width: 44, side: "R", kind: "minutes", value: (r) => r.sim_inst_min },
    { key: "dual", label: "Dual rcvd.", width: 44, side: "R", kind: "minutes", value: (r) => r.dual_min },
    { key: "pic", label: "PIC", width: 46, side: "R", kind: "minutes", value: (r) => r.pic_min },
    { key: "total", label: "Total duration", width: 52, side: "R", kind: "minutes", value: (r) => r.total_min },
  ],
  easa: [
    { key: "date", label: "Date", width: 62, side: "L", kind: "text", value: date },
    { key: "dep", label: "Place", group: "Departure", width: 44, side: "L", kind: "text", value: (r) => r.from_icao },
    { key: "dep_t", label: "Time", group: "Departure", width: 38, side: "L", kind: "text", value: (r) => hhmmZ(r.out_ms) },
    { key: "arr", label: "Place", group: "Arrival", width: 44, side: "L", kind: "text", value: (r) => r.to_icao },
    { key: "arr_t", label: "Time", group: "Arrival", width: 38, side: "L", kind: "text", value: (r) => hhmmZ(r.in_ms) },
    { key: "type", label: "Make, model", group: "Aircraft", width: 52, side: "L", kind: "text", value: (r) => r.aircraft_type },
    { key: "reg", label: "Registration", group: "Aircraft", width: 60, side: "L", kind: "text", value: (r) => r.ident },
    { key: "se", label: "SE", group: "Single-pilot time", width: 44, side: "L", kind: "minutes", value: (r) => (is(r, "SEL", "SES") || (is(r, "RH") && r.engine_count <= 1) ? r.total_min : 0) },
    { key: "me", label: "ME", group: "Single-pilot time", width: 44, side: "L", kind: "minutes", value: (r) => (is(r, "MEL", "MES") || (is(r, "RH") && r.engine_count > 1) ? r.total_min : 0) },
    { key: "mp", label: "Multi-pilot time", width: 42, side: "L", kind: "minutes", value: () => 0 },
    { key: "total", label: "Total time of flight", width: 50, side: "L", kind: "minutes", value: (r) => r.total_min },

    { key: "picname", label: "Name of PIC", width: 64, side: "R", kind: "text", value: () => "Self" },
    { key: "ldg_day", label: "Day", group: "Landings", width: 36, side: "R", kind: "count", value: (r) => r.day_landings },
    { key: "ldg_night", label: "Night", group: "Landings", width: 36, side: "R", kind: "count", value: (r) => r.night_landings },
    { key: "night", label: "Night", group: "Condition time", width: 44, side: "R", kind: "minutes", value: (r) => r.night_min },
    { key: "ifr", label: "IFR", group: "Condition time", width: 44, side: "R", kind: "minutes", value: (r) => r.ifr_min },
    { key: "pic", label: "PIC", group: "Pilot function time", width: 44, side: "R", kind: "minutes", value: (r) => r.pic_min },
    { key: "cop", label: "Co-pilot", group: "Pilot function time", width: 44, side: "R", kind: "minutes", value: () => 0 },
    { key: "dual", label: "Dual", group: "Pilot function time", width: 44, side: "R", kind: "minutes", value: (r) => r.dual_min },
    { key: "remarks", label: "Remarks and endorsements", width: 0, side: "R", kind: "text", value: (r) => r.remarks, flex: true },
  ],
};

/** Print a column's value: decimal hours (FAA) or h:mm (EASA); zero is blank. */
export function printValue(col: Col, v: string | number, layout: Layout): string {
  if (col.kind === "text") return String(v ?? "");
  const n = Number(v) || 0;
  if (n === 0) return "";
  if (col.kind === "count") return String(n);
  if (layout === "faa") return (n / 60).toFixed(1);
  return `${Math.floor(n / 60)}:${String(n % 60).padStart(2, "0")}`;
}

/** Group consecutive columns sharing a group label, for the header's top tier. */
export function groups(cols: Col[]): { label: string; span: number; width: number }[] {
  const out: { label: string; span: number; width: number }[] = [];
  for (const c of cols) {
    const last = out[out.length - 1];
    if (c.group && last && last.label === c.group) {
      last.span++;
      last.width += c.width;
    } else {
      out.push({ label: c.group ?? "", span: 1, width: c.width });
    }
  }
  return out;
}

/** The simple list view: the columns most pilots look at, in one table. */
export const LIST_COLS: Col[] = [
  { key: "from", label: "From", width: 0, side: "L", kind: "text", value: (r) => r.from_icao },
  { key: "to", label: "To", width: 0, side: "L", kind: "text", value: (r) => r.to_icao },
  { key: "type", label: "Aircraft", width: 0, side: "L", kind: "text", value: (r) => r.aircraft_type },
  { key: "ident", label: "Registration", width: 0, side: "L", kind: "text", value: (r) => r.ident },
  { key: "block", label: "Out – In (UTC)", width: 0, side: "L", kind: "text", value: (r) => (r.out_ms != null && r.in_ms != null ? `${hhmmZ(r.out_ms)} – ${hhmmZ(r.in_ms)}` : "") },
  { key: "total", label: "Total", width: 0, side: "R", kind: "minutes", value: (r) => r.total_min },
  { key: "night", label: "Night", width: 0, side: "R", kind: "minutes", value: (r) => r.night_min },
  { key: "ifr", label: "Instrument", width: 0, side: "R", kind: "minutes", value: (r) => r.sim_inst_min + r.act_inst_min },
  { key: "ldg", label: "Landings", width: 0, side: "R", kind: "count", value: (r) => r.day_landings + r.night_landings },
  { key: "remarks", label: "Remarks", width: 0, side: "R", kind: "text", value: (r) => r.remarks, flex: true },
];
