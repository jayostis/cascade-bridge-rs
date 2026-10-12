/// <reference lib="esnext.disposable" />
import type { Adapter, Named } from "./cascade_bridge.js";

export * from "./cascade_bridge.js";
export { default } from "./cascade_bridge.js";

declare module "./cascade_bridge.js" {
  interface Adapter {
    /**
     * The adapter as loaded, for `restore` in a later module instance. The image is the instance's
     * whole memory, so take it straight after the load, in an instance that made no other adapter:
     * any other throws.
     */
    snapshot(): Uint8Array;
  }
}

/**
 * The adapter a snapshot holds, or `undefined` where it is not for this build of the module, these
 * IRIs and these files, or is not whole: load as usual then. Only the files the load read are
 * compared, so `adapter` and `vocabulary` may hold more.
 *
 * The image replaces the module instance's whole memory, so this throws in an instance that has
 * already made an adapter, by `Adapter.load` or `restore`. One restored adapter per instance;
 * `Adapter.load` after it is fine.
 */
export function restore(snapshot: Uint8Array, adapter: Named, vocabulary?: Named): Adapter | undefined;

/** A file name for a snapshot: the start of the digest of the files it was made from. */
export function snapshotName(snapshot: Uint8Array): string;
