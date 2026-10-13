/** Load occt-import-js (UMD) from vendor for ESM apps. */
let occtPromise = null;

export function loadOcct() {
  if (occtPromise) return occtPromise;

  occtPromise = new Promise((resolve, reject) => {
    const url = new URL("./vendor/occt-import-js.js", import.meta.url).href;

    if (typeof globalThis.occtimportjs === "function") {
      globalThis
        .occtimportjs({
          locateFile: (file) => new URL(`./vendor/${file}`, import.meta.url).href,
        })
        .then(resolve)
        .catch(reject);
      return;
    }

    const script = document.createElement("script");
    script.src = url;
    script.async = true;
    script.onload = () => {
      const factory = globalThis.occtimportjs;
      if (typeof factory !== "function") {
        reject(new Error("occt-import-js failed to initialize"));
        return;
      }
      factory({
        locateFile: (file) => new URL(`./vendor/${file}`, import.meta.url).href,
      })
        .then(resolve)
        .catch(reject);
    };
    script.onerror = () => reject(new Error("failed to load occt-import-js.js"));
    document.head.appendChild(script);
  });

  return occtPromise;
}
