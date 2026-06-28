/**
 * Browser smoke test — layout fill, WASM analysis, data panels.
 * Run: node e2e.mjs [baseUrl]
 */
import { chromium } from "playwright";

const BASE = process.argv[2] ?? "http://localhost:8765";
const TIMEOUT = 90_000;

const REQUIRED_IDS = [
  "stage",
  "workspace",
  "metrics",
  "canvas-3d",
  "canvas-xy",
  "canvas-xz",
  "canvas-stock",
  "features-out",
  "path-stats",
  "code-out",
  "m-records",
  "m-cut",
  "m-nc",
];

function assert(cond, msg) {
  if (!cond) throw new Error(msg);
}

async function main() {
  const browser = await chromium.launch({ headless: true });
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
  const errors = [];
  page.on("pageerror", (e) => errors.push(`pageerror: ${e.message}`));
  page.on("console", (msg) => {
    if (msg.type() === "error") errors.push(`console: ${msg.text()}`);
  });

  try {
    await page.goto(`${BASE}/index.html`, { waitUntil: "networkidle", timeout: TIMEOUT });

    for (const id of REQUIRED_IDS) {
      assert(await page.locator(`#${id}`).count(), `Missing #${id}`);
    }

    // WASM ready
    await page.waitForFunction(
      () => !document.getElementById("message")?.textContent?.includes("make wasm"),
      { timeout: TIMEOUT }
    );

    await page.click('[data-sample="cylinder_block.step"]');
    await page.waitForFunction(
      () => document.body.classList.contains("has-results"),
      { timeout: TIMEOUT }
    );
    await page.waitForFunction(
      () => (document.querySelector("#code-out code")?.textContent?.length ?? 0) > 100,
      { timeout: TIMEOUT }
    );
    await page.waitForFunction(
      () => document.getElementById("m-nc")?.textContent === "Pass",
      { timeout: TIMEOUT }
    );

    // Layout: workspace must fill most of viewport (grid bug regression)
    const layout = await page.evaluate(() => {
      const stage = document.getElementById("stage");
      const workspace = document.getElementById("workspace");
      const canvas3d = document.getElementById("canvas-3d");
      const shell = document.querySelector(".shell");
      return {
        stageH: stage?.clientHeight ?? 0,
        workspaceH: workspace?.clientHeight ?? 0,
        shellH: shell?.clientHeight ?? 0,
        canvas3dW: canvas3d?.clientWidth ?? 0,
        canvas3dH: canvas3d?.clientHeight ?? 0,
        canvasXYW: document.getElementById("canvas-xy")?.clientWidth ?? 0,
        canvasStockH: document.getElementById("canvas-stock")?.clientHeight ?? 0,
        codeH: document.getElementById("code-wrap")?.clientHeight ?? 0,
      };
    });

    assert(layout.shellH > 800, `Shell too short: ${layout.shellH}`);
    assert(layout.stageH > layout.shellH * 0.7, `Stage not filling shell: ${layout.stageH}/${layout.shellH}`);
    assert(layout.workspaceH > layout.stageH * 0.95, `Workspace not filling stage: ${layout.workspaceH}/${layout.stageH}`);
    assert(layout.canvas3dW > 400, `3D canvas too narrow: ${layout.canvas3dW}`);
    assert(layout.canvas3dH > 250, `3D canvas too short: ${layout.canvas3dH}`);
    assert(layout.canvasXYW > 80, `XY canvas too narrow: ${layout.canvasXYW}`);
    assert(layout.canvasStockH > 80, `Stock canvas too short: ${layout.canvasStockH}`);
    assert(layout.codeH > 100, `Code panel too short: ${layout.codeH}`);

    // Metrics populated
    const metrics = await page.evaluate(() => ({
      records: document.getElementById("m-records")?.textContent,
      cut: document.getElementById("m-cut")?.textContent,
      nc: document.getElementById("m-nc")?.textContent,
      features: document.getElementById("features-out")?.textContent,
      pathStats: document.getElementById("path-stats")?.textContent,
      gcodeLen: document.querySelector("#code-out code")?.textContent?.length ?? 0,
      gcodeHtml: document.querySelector("#code-out code")?.innerHTML?.includes("gc-g"),
      fileMeta: document.getElementById("file-meta")?.textContent,
    }));

    assert(metrics.records === "27", `Expected 27 entities, got ${metrics.records}`);
    assert(metrics.cut?.includes("mm"), `Cut length missing: ${metrics.cut}`);
    assert(metrics.nc === "Pass", `NC validation expected Pass, got ${metrics.nc}`);
    assert(metrics.features?.includes("Coaxial"), `Feature label missing: ${metrics.features}`);
    assert(metrics.pathStats?.includes("Segments"), `Path stats missing: ${metrics.pathStats}`);
    assert(metrics.gcodeLen > 100, `G-code too short: ${metrics.gcodeLen}`);
    assert(metrics.gcodeHtml, "G-code syntax highlighting not applied");
    assert(metrics.fileMeta?.includes("cylinder_block.step"), `File meta missing: ${metrics.fileMeta}`);

    // Tab switch
    await page.click('.tab[data-tab="toolpath"]');
    await page.waitForTimeout(200);
    const pathJson = await page.textContent("#code-out code");
    assert(pathJson?.includes('"segments"'), "Toolpath tab did not render JSON");

    // Copy button (clipboard may be blocked in headless — verify click does not throw)
    await page.click('.tab[data-tab="gcode"]');
    await page.waitForTimeout(150);
    await page.click("#copy-code");
    await page.waitForTimeout(300);

    const blocking = errors.filter(
      (e) => !e.includes("favicon") && !e.includes("Failed to load resource")
    );
    assert(blocking.length === 0, `Browser errors:\n${blocking.join("\n")}`);

    await page.screenshot({ path: "/Users/vladimirgutierrez/Documents/projects/step-rs/web/e2e-screenshot.png", fullPage: false });
    console.log("PASS — layout, WASM analysis, metrics, features, G-code, tabs");
    console.log(JSON.stringify({ layout, metrics: { ...metrics, gcodeLen: metrics.gcodeLen } }, null, 2));
  } finally {
    await browser.close();
  }
}

main().catch((e) => {
  console.error("FAIL —", e.message);
  process.exit(1);
});
