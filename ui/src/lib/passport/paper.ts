// Passport paper: a cream page with fine guilloche lines, as on real
// security printing. Generated once as an SVG data URL.

function guilloche(w: number, h: number, colors: string[]): string {
  const lines: string[] = [];
  const rows = 26;
  for (let i = 0; i < rows; i++) {
    const y0 = (i / rows) * h;
    const amp = 6 + (i % 3) * 3;
    const freq = (2 * Math.PI) / (60 + (i % 4) * 14);
    const phase = i * 0.7;
    let d = `M0 ${y0.toFixed(1)}`;
    for (let x = 4; x <= w; x += 4) {
      const y = y0 + Math.sin(x * freq + phase) * amp + Math.sin(x * freq * 0.37 + phase * 2) * amp * 0.6;
      d += ` L${x} ${y.toFixed(1)}`;
    }
    lines.push(`<path d="${d}" fill="none" stroke="${colors[i % colors.length]}" stroke-width="0.6" opacity="0.55"/>`);
  }
  // A faint rosette, drawn as a closed curve through the center so there
  // is no empty disc in the middle of the page.
  const cx = w / 2;
  const cy = h * 0.46;
  for (let k = 0; k < 3; k++) {
    let d = "";
    const r0 = w * (0.2 + k * 0.05);
    for (let i = 0; i <= 720; i++) {
      const a = (i / 720) * Math.PI * 2;
      const r = r0 * (0.55 + 0.45 * Math.cos(a * (11 + k * 2)));
      d += `${i ? "L" : "M"}${(cx + Math.cos(a) * r).toFixed(1)} ${(cy + Math.sin(a) * r).toFixed(1)} `;
    }
    lines.push(`<path d="${d}Z" fill="none" stroke="${colors[k % colors.length]}" stroke-width="0.5" opacity="0.4"/>`);
  }
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="${w}" height="${h}" viewBox="0 0 ${w} ${h}">${lines.join("")}</svg>`;
  return `url("data:image/svg+xml;utf8,${encodeURIComponent(svg)}")`;
}

let cached: string | null = null;
export function paperPattern(): string {
  cached ??= guilloche(360, 510, ["#9fc1bd", "#d3a9b8", "#b6c59a"]);
  return cached;
}

/** ICAO 9303 check digit. */
export function checkDigit(s: string): string {
  const weights = [7, 3, 1];
  let sum = 0;
  for (let i = 0; i < s.length; i++) {
    const c = s[i];
    const v = c >= "0" && c <= "9" ? +c : c >= "A" && c <= "Z" ? c.charCodeAt(0) - 55 : 0;
    sum += v * weights[i % 3];
  }
  return String(sum % 10);
}

const mrzText = (s: string) =>
  s
    .normalize("NFD")
    .replace(/[̀-ͯ]/g, "")
    .toUpperCase()
    .replace(/[^A-Z0-9]+/g, "<");

const yymmdd = (ms: number) => {
  const d = new Date(ms);
  return `${String(d.getUTCFullYear() % 100).padStart(2, "0")}${String(d.getUTCMonth() + 1).padStart(2, "0")}${String(d.getUTCDate()).padStart(2, "0")}`;
};

/** The two 44-character lines at the foot of a passport data page. */
export function mrz(opts: { name: string; nationality: string; number: string; issued: number }): [string, string] {
  const parts = opts.name.trim().split(/\s+/).filter(Boolean);
  const surname = parts.length > 1 ? parts[parts.length - 1] : (parts[0] ?? "PILOT");
  const given = parts.length > 1 ? parts.slice(0, -1).join(" ") : "";
  const state = /^[A-Za-z]{3}$/.test(opts.nationality) ? opts.nationality.toUpperCase() : "UTO";
  const line1 = `P<${state}${mrzText(surname)}<<${mrzText(given)}`.padEnd(44, "<").slice(0, 44);
  const num = mrzText(opts.number).padEnd(9, "<").slice(0, 9);
  const born = yymmdd(opts.issued);
  const expires = yymmdd(opts.issued + 10 * 365.25 * 86_400_000);
  const personal = "<".repeat(14);
  const body = `${num}${checkDigit(num)}${state}${born}${checkDigit(born)}<${expires}${checkDigit(expires)}${personal}${checkDigit(personal)}`;
  const composite = `${num}${checkDigit(num)}${born}${checkDigit(born)}${expires}${checkDigit(expires)}${personal}${checkDigit(personal)}`;
  return [line1, `${body}${checkDigit(composite)}`];
}

/** A stable passport number from the pilot ID, or made up from the name. */
export function passportNumber(pilotId: string, name: string): string {
  if (pilotId.trim()) return mrzText(pilotId).replace(/</g, "").slice(0, 9) || "FD0000001";
  let h = 2166136261;
  for (const c of name || "pilot") h = Math.imul(h ^ c.charCodeAt(0), 16777619);
  return `FD${String((h >>> 0) % 10_000_000).padStart(7, "0")}`;
}

let holoCached: string | null = null;
/** The hologram's line art, used as a mask: only these thin lines light up
 * over the photo, so the photo itself is never washed out. A rosette, rings
 * and wavy guilloche lines, like the security film on a real passport. */
export function holoLines(): string {
  if (holoCached) return holoCached;
  const w = 108;
  const h = 140;
  const p: string[] = [];
  const line = (d: string, sw = 0.5, op = 1) =>
    p.push(`<path d="${d}" fill="none" stroke="#fff" stroke-width="${sw}" opacity="${op}"/>`);
  // Rings around a point up and to the right of the face.
  const rx = w * 0.7;
  const ry = h * 0.3;
  for (let r = 6; r < 120; r += 5.5) {
    p.push(`<circle cx="${rx}" cy="${ry}" r="${r}" fill="none" stroke="#fff" stroke-width="0.45" opacity="0.8"/>`);
  }
  // A rosette in the lower left.
  const cx = w * 0.28;
  const cy = h * 0.74;
  for (let k = 0; k < 2; k++) {
    let d = "";
    for (let i = 0; i <= 360; i++) {
      const a = (i / 360) * Math.PI * 2;
      const r = 26 * (0.55 + 0.45 * Math.cos(a * (9 + k * 4)));
      d += `${i ? "L" : "M"}${(cx + Math.cos(a) * r).toFixed(1)} ${(cy + Math.sin(a) * r).toFixed(1)}`;
    }
    line(d + "Z", 0.5, 0.9);
  }
  // Wavy lines across the whole photo.
  for (let i = 0; i < 9; i++) {
    const y0 = 8 + i * 16;
    let d = `M0 ${y0}`;
    for (let x = 2; x <= w; x += 2) {
      d += `L${x} ${(y0 + Math.sin(x * 0.11 + i) * 3.2 + Math.sin(x * 0.037 + i * 2) * 2).toFixed(1)}`;
    }
    line(d, 0.4, 0.7);
  }
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="${w}" height="${h}" viewBox="0 0 ${w} ${h}">${p.join("")}</svg>`;
  holoCached = `url("data:image/svg+xml;utf8,${encodeURIComponent(svg)}")`;
  return holoCached;
}
