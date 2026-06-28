/**
 * Browser smoke test — coaxial grid + system test status.
 * Run: node e2e.mjs [baseUrl]
 */
import { chromium } from "playwright";

const BASE = process.argv[2] ?? "http://localhost:8765";
const TIMEOUT = 120_000;

const REQUIRED_IDS = [
  "workspace",
  "coaxial-table",
  "coaxial-table-body",
  "export-csv",
  "ip-mask-toggle",
  "canvas-3d",
  "viz",
  "system-tests-list",
  "m-coaxial",
  "m-records",
];

function assert(cond, msg) {
  if (!cond) throw new Error(msg);
}

async function main() {
  const browser = await chromium.launch({ headless: true });
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
  page.setDefaultTimeout(TIMEOUT);
  await page.emulateMedia({ reducedMotion: "reduce" });
  const errors = [];
  page.on("pageerror", (e) => errors.push(`pageerror: ${e.message}`));
  page.on("console", (msg) => {
    if (msg.type() === "error") errors.push(`console: ${msg.text()}`);
  });

  try {
    await page.goto(`${BASE}/index.html`, { waitUntil: "networkidle" });

    for (const id of REQUIRED_IDS) {
      assert(await page.locator(`#${id}`).count(), `Missing #${id}`);
    }

    await page.waitForFunction(() =>
      !document.getElementById("message")?.textContent?.includes("make wasm")
    );

    await page.waitForFunction(() => {
      const rows = document.querySelectorAll("#system-tests-landing-list .system-test-row.pass");
      return rows.length >= 5;
    });

    await page.click('[data-sample="cylinder_block.step"]');
    await page.waitForFunction(() => document.body.classList.contains("has-results"));
    await page.waitForFunction(
      () => document.querySelectorAll("#coaxial-table-body tr").length >= 1
    );

    const state = await page.evaluate(() => {
      const rows = [...document.querySelectorAll("#coaxial-table-body tr")].map((tr) =>
        [...tr.querySelectorAll("td")].map((td) => td.textContent?.trim())
      );
      const tests = [...document.querySelectorAll("#system-tests-list .system-test-row")].map(
        (li) => ({
          pass: li.classList.contains("pass"),
          text: li.textContent?.trim(),
        })
      );
      return {
        coaxial: document.getElementById("m-coaxial")?.textContent,
        records: document.getElementById("m-records")?.textContent,
        rows,
        tests,
        tableH: document.querySelector(".coaxial-table-wrap")?.clientHeight ?? 0,
        workspaceH: document.getElementById("workspace")?.clientHeight ?? 0,
      };
    });

    assert(state.records === "27", `Expected 27 entities, got ${state.records}`);
    assert(Number(state.coaxial) >= 1, `Expected coaxial clusters, got ${state.coaxial}`);
    assert(state.rows.some((r) => r[2]?.includes("Coaxial")), `Coaxial label missing: ${JSON.stringify(state.rows)}`);
    assert(state.rows[0]?.length >= 13, `Expected dense columns, got ${state.rows[0]?.length}`);
    assert(state.tests.filter((t) => t.pass).length >= 5, `System tests not all pass: ${JSON.stringify(state.tests)}`);
    assert(state.tableH > 200, `Table too short: ${state.tableH}`);

    const viewer = await page.evaluate(() => ({
      w: document.getElementById("canvas-3d")?.clientWidth ?? 0,
      h: document.getElementById("canvas-3d")?.clientHeight ?? 0,
    }));
    assert(viewer.w > 200, `3D canvas too narrow: ${viewer.w}`);
    assert(viewer.h > 200, `3D canvas too short: ${viewer.h}`);

    const [download] = await Promise.all([
      page.waitForEvent("download"),
      page.click("#export-csv"),
    ]);
    assert(
      download.suggestedFilename() === "steprs-coaxial-holes.csv",
      `Unexpected download: ${download.suggestedFilename()}`
    );

    await page.check("#ip-mask-toggle");
    await page.waitForFunction(() => !document.getElementById("manifest-panel")?.classList.contains("hidden"));
    const manifest = await page.textContent("#manifest-preview");
    assert(manifest?.includes("coaxial_step_bore"), "ITAR manifest should show coaxial_step_bore");
    assert(!manifest?.includes("axis_origin"), "ITAR manifest leaked spatial metadata");

    const [masked] = await Promise.all([
      page.waitForEvent("download"),
      page.click("#export-csv"),
    ]);
    assert(
      masked.suggestedFilename() === "steprs-structural-manifest.json",
      `Unexpected masked download: ${masked.suggestedFilename()}`
    );
    const manifestPath = await masked.path();
    const manifestFile = await (await import("node:fs/promises")).readFile(manifestPath, "utf8");
    assert(!manifestFile.includes("axis_origin"), "Manifest leaked spatial metadata");
    assert(manifestFile.includes("coaxial_step_bore"), "Manifest missing classification");
    assert(manifestFile.includes("volume_limit_mm3"), "Manifest missing volumetric limits");

    const blocking = errors.filter(
      (e) => !e.includes("favicon") && !e.includes("Failed to load resource")
    );
    assert(blocking.length === 0, `Browser errors:\n${blocking.join("\n")}`);

    console.log("PASS — coaxial grid, system tests, CSV export");
    console.log(JSON.stringify(state, null, 2));
  } finally {
    await browser.close();
  }
}

main().catch((e) => {
  console.error("FAIL —", e.message);
  process.exit(1);
});
