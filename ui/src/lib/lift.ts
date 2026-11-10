// Raising a map marker off the ground in the 3D view: the aircraft drawn
// at the height of its 3D path (the wall), with a thin stem down to its
// spot on the ground. MapLibre places markers on the ground only, so an
// invisible custom layer keeps the camera's matrix for each frame and the
// marker is moved up by the on-screen difference between the ground point
// and the raised one. It draws nothing, so the graphics card does no work.

import maplibregl from "maplibre-gl";

type Mat = ArrayLike<number>;

let matrix: Mat | null = null;

/** Add once per map: keeps the camera matrix (draws nothing). */
export function addLiftLayer(map: maplibregl.Map) {
  if (map.getLayer("fd-lift")) return;
  map.addLayer({
    id: "fd-lift",
    type: "custom",
    renderingMode: "3d",
    render(_gl, args) {
      matrix = args.defaultProjectionData.mainMatrix as unknown as Mat;
    },
  });
}

function screen(map: maplibregl.Map, lng: number, lat: number, meters: number): [number, number] | null {
  const m = matrix;
  if (!m) return null;
  const c = maplibregl.MercatorCoordinate.fromLngLat([lng, lat], meters);
  const x = c.x;
  const y = c.y;
  const z = c.z;
  const cx = m[0] * x + m[4] * y + m[8] * z + m[12];
  const cy = m[1] * x + m[5] * y + m[9] * z + m[13];
  const w = m[3] * x + m[7] * y + m[11] * z + m[15];
  if (!(w > 0)) return null;
  const el = map.getCanvas();
  return [((cx / w + 1) / 2) * el.clientWidth, ((1 - cy / w) / 2) * el.clientHeight];
}

/**
 * A marker that can stand above the ground: `set(lng, lat, meters)` each
 * time it moves (0 = on the ground), `update()` on every map frame.
 */
export class Raised {
  private stem: maplibregl.Marker;
  private at: [number, number] = [0, 0];
  private meters = 0;
  constructor(
    private map: maplibregl.Map,
    private marker: maplibregl.Marker,
  ) {
    const el = document.createElement("div");
    el.className = "lift-stem";
    el.innerHTML = '<i class="lift-line"></i><i class="lift-dot"></i>';
    this.stem = new maplibregl.Marker({ element: el, anchor: "center" });
  }
  set(lng: number, lat: number, meters: number) {
    this.at = [lng, lat];
    this.meters = meters;
    this.update();
  }
  update() {
    const [lng, lat] = this.at;
    const up = this.meters > 1 ? screen(this.map, lng, lat, this.meters) : null;
    const down = up ? screen(this.map, lng, lat, 0) : null;
    if (!up || !down) {
      this.marker.setOffset([0, 0]);
      this.stem.remove();
      return;
    }
    const dx = up[0] - down[0];
    const dy = up[1] - down[1];
    this.marker.setOffset([dx, dy]);
    const len = Math.hypot(dx, dy);
    if (len < 4) {
      this.stem.remove();
      return;
    }
    const line = this.stem.getElement().firstElementChild as HTMLElement;
    line.style.width = `${len}px`;
    line.style.transform = `rotate(${Math.atan2(dy, dx)}rad)`;
    this.stem.setLngLat([lng, lat]);
    if (!this.stem.getElement().isConnected) this.stem.addTo(this.map);
  }
  remove() {
    this.stem.remove();
    this.marker.setOffset([0, 0]);
  }
}
