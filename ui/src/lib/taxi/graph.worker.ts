// Builds the taxi graph off the main thread (a big airport can take a
// couple of hundred milliseconds), so the map never stutters.
import { buildGraph, type LL, type RawFeature } from "./graph";

self.onmessage = (e: MessageEvent<{ raw: RawFeature[]; origin: LL }>) => {
  self.postMessage(buildGraph(e.data.raw, e.data.origin));
};
