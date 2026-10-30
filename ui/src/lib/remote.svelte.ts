// FlightDeck in a browser on another device: calls go to the flight sim
// PC over the home network, and live updates arrive on a WebSocket.

export const inRemote =
  typeof window !== "undefined" && !("__TAURI_INTERNALS__" in window) && !!(window as unknown as { __FD_REMOTE__?: boolean }).__FD_REMOTE__;

const KEY = "flightdeck.token";

function token(): string | null {
  try {
    return localStorage.getItem(KEY);
  } catch {
    return null;
  }
}

/** Whether this device is paired, and whether it may make changes. */
export const session = $state<{ checked: boolean; paired: boolean; canEdit: boolean; version: string; offline: boolean }>({
  checked: false,
  paired: false,
  canEdit: false,
  version: "",
  offline: false,
});

const auth = (): Record<string, string> => {
  const t = token();
  return t ? { Authorization: `Bearer ${t}` } : {};
};

export async function checkSession() {
  try {
    const r = await fetch("/api/session", { headers: auth() });
    const s = await r.json();
    session.paired = !!s.paired;
    session.canEdit = !!s.can_edit;
    session.version = s.version ?? "";
    session.offline = false;
  } catch {
    session.offline = true;
  }
  session.checked = true;
}

export async function pair(code: string): Promise<string | null> {
  const name = guessName();
  const r = await fetch("/api/pair", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ code, name }),
  });
  if (!r.ok) return (await r.text()) || "Pairing failed.";
  const { token: t } = await r.json();
  try {
    localStorage.setItem(KEY, t);
  } catch {
    return "This browser does not allow saving the pairing (private mode?).";
  }
  await checkSession();
  return null;
}

/** A friendly name for this device, shown on the PC. */
function guessName(): string {
  const ua = navigator.userAgent;
  const device = /iPad/.test(ua) ? "iPad" : /iPhone/.test(ua) ? "iPhone" : /Android/.test(ua) ? (/Mobile/.test(ua) ? "Android phone" : "Android tablet") : /Mac/.test(ua) ? "Mac" : /Windows/.test(ua) ? "Windows PC" : /Linux/.test(ua) ? "Linux computer" : "Device";
  const browser = /Edg\//.test(ua) ? "Edge" : /Chrome\//.test(ua) ? "Chrome" : /Firefox\//.test(ua) ? "Firefox" : /Safari\//.test(ua) ? "Safari" : "browser";
  return `${device}, ${browser}`;
}

export async function remoteCall<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const r = await fetch(`/api/call/${cmd}`, {
    method: "POST",
    headers: { "Content-Type": "application/json", ...auth() },
    body: JSON.stringify(args ?? {}),
  });
  if (r.status === 401) {
    session.paired = false;
    throw new Error("This device is no longer paired with FlightDeck.");
  }
  const body = await r.json().catch(() => null);
  if (!r.ok) throw (body && body.error) || `FlightDeck said ${r.status}`;
  return body as T;
}

// --- live events ----------------------------------------------------------

type Cb = (payload: unknown) => void;
const listeners = new Map<string, Set<Cb>>();
let socket: WebSocket | null = null;
let retry = 1000;

function connect() {
  const t = token();
  if (!t || socket) return;
  const proto = location.protocol === "https:" ? "wss" : "ws";
  const ws = new WebSocket(`${proto}://${location.host}/api/events?token=${encodeURIComponent(t)}`);
  socket = ws;
  ws.onopen = () => {
    retry = 1000;
    session.offline = false;
    // Catch up on anything missed while disconnected.
    for (const cb of listeners.get("reconnected") ?? []) cb(null);
  };
  ws.onmessage = (m) => {
    try {
      const { event, payload } = JSON.parse(m.data);
      for (const cb of listeners.get(event) ?? []) cb(payload);
    } catch {
      /* ignore */
    }
  };
  ws.onclose = () => {
    socket = null;
    session.offline = true;
    setTimeout(connect, retry);
    retry = Math.min(retry * 2, 15000);
  };
}

export function remoteListen(event: string, cb: Cb): () => void {
  let set = listeners.get(event);
  if (!set) listeners.set(event, (set = new Set()));
  set.add(cb);
  connect();
  return () => set!.delete(cb);
}
