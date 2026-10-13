/**
 * Browser smoke test - sample load, bore table, viewer mesh, CSV.
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
  "canvas-3d",
  "viz",
  "m-coaxial",
  "m-time",
  "landing",
  "landing-cta",
  "viewer-status",
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

    assert(await page.locator(".landing-brand").count(), "Missing brand hero");
    assert(await page.locator(".landing-faq").count(), "Missing crawlable FAQ");
    assert(await page.locator('script[type="application/ld+json"]').count(), "Missing JSON-LD");

    await page.waitForFunction(() =>
      !document.getElementById("message")?.textContent?.includes("make wasm")
    );

    await page.click('[data-sample="ap214_compact.step"]');
    await page.waitForFunction(() => document.body.classList.contains("has-results"));
    await page.waitForFunction(
      () => document.querySelectorAll("#coaxial-table-body tr").length >= 1
    );
    await page.waitForFunction(
      () => Number(document.getElementById("canvas-3d")?.dataset?.meshes) > 0,
      null,
      { timeout: 60_000 }
    );

    const state = await page.evaluate(() => {
      const rows = [...document.querySelectorAll("#coaxial-table-body tr")].map((tr) =>
        [...tr.querySelectorAll("td")].map((td) => td.textContent?.trim())
      );
      return {
        coaxial: document.getElementById("m-coaxial")?.textContent,
        time: document.getElementById("m-time")?.textContent,
        rows,
        brand: document.querySelector(".landing-brand")?.textContent,
        tableH: document.querySelector(".coaxial-table-wrap")?.clientHeight ?? 0,
        workspaceH: document.getElementById("workspace")?.clientHeight ?? 0,
      };
    });

    assert(Number(state.coaxial) >= 1, `Expected bores, got ${state.coaxial}`);
    assert(state.time?.includes("ms"), `Expected parse time, got ${state.time}`);
    assert(
      state.rows.some((r) => r[2]?.toLowerCase().includes("coaxial") || r[1] === "blind"),
      `Bore row missing: ${JSON.stringify(state.rows)}`
    );
    assert(state.rows[0]?.length === 6, `Expected 6 columns, got ${state.rows[0]?.length}`);
    assert(state.tableH > 160, `Table too short: ${state.tableH}`);

    await page.keyboard.press("ArrowDown");
    const selected = await page.evaluate(() => {
      const rows = [...document.querySelectorAll("#coaxial-table-body tr")];
      return rows.findIndex((tr) => tr.classList.contains("selected"));
    });
    assert(selected === 1, `ArrowDown should select row 1, got ${selected}`);

    const viewer = await page.evaluate(() => ({
      w: document.getElementById("canvas-3d")?.clientWidth ?? 0,
      h: document.getElementById("canvas-3d")?.clientHeight ?? 0,
      meshes: document.getElementById("canvas-3d")?.dataset?.meshes,
    }));
    assert(viewer.w > 200, `3D canvas too narrow: ${viewer.w}`);
    assert(viewer.h > 200, `3D canvas too short: ${viewer.h}`);
    assert(Number(viewer.meshes) > 0, `Expected viewer mesh, got ${viewer.meshes}`);

    const [download] = await Promise.all([
      page.waitForEvent("download"),
      page.click("#export-csv"),
    ]);
    assert(
      download.suggestedFilename() === "steprs-coaxial-holes.csv",
      `Unexpected download: ${download.suggestedFilename()}`
    );

    const blocking = errors.filter(
      (e) => !e.includes("favicon") && !e.includes("Failed to load resource")
    );
    assert(blocking.length === 0, `Browser errors:\n${blocking.join("\n")}`);

    console.log("PASS - bore table, keyboard select, viewer, CSV");
    console.log(JSON.stringify(state, null, 2));
  } finally {
    await browser.close();
  }
}

main().catch((e) => {
  console.error("FAIL -", e.message);
  process.exit(1);
});
