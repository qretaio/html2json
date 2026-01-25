// Auto-initializing wrapper for convenience
import _init from "./html2json.js";
import { extract as _extract, initSync } from "./html2json.js";

let initPromise;

// Auto-init on first call
function init() {
  if (!initPromise) {
    initPromise = _init();
  }
  return initPromise;
}

// Export auto-initialized extract (async for first call)
export async function extract(html, spec_json) {
  await init();
  return _extract(html, spec_json);
}

// Also export init functions for those who want to control timing
export { init, initSync };

// Export raw extract for advanced use (sync, requires manual init)
export { _extract as extractSync };

export default init;
