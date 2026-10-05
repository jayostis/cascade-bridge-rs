import { readFileSync } from "node:fs";
import { initSync } from "./cascade_bridge.js";

initSync({ module: readFileSync(new URL("./cascade_bridge_bg.wasm", import.meta.url)) });

export * from "./cascade_bridge.js";
export { default } from "./cascade_bridge.js";
