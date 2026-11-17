import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { randomBytes, randomUUID } from "node:crypto";

const [base, main] = process.argv.slice(2);
const key = randomBytes(32).toString("hex");

async function workspace(body, token = key, status = 200) {
  const response = await fetch(new URL("/api/workspace", base), {
    method: body ? "POST" : "GET",
    headers: {
      "Content-Type": "application/json",
      Authorization: `Bearer ${token}`,
    },
    body: body ? JSON.stringify(body) : undefined,
  });
  assert.equal(response.status, status, await response.clone().text());
  if (status === 200) return response.json();
}

const initial = await workspace();
const project = await workspace({
  revision: initial.revision,
  action: "create_project",
  name: "松庭公寓（虚构）",
  requirements: "内部记录：仅用于试验",
});
const line = {
  name: "示例开关",
  category: "equipment",
  quantity: 3,
  unit_cents: 1999,
};
const catalog = await workspace({
  revision: project.revision,
  action: "save_product",
  item: line,
});
const first = await workspace({
  revision: catalog.revision,
  action: "save_quote",
  project: 0,
  lines: [catalog.workspace.catalog[0]],
  publish: false,
});
assert.equal(first.workspace.projects[0].draft.total_cents, 1999);
const published = await workspace({
  revision: first.revision,
  action: "save_quote",
  project: 0,
  lines: [
    line,
    { name: "调试", category: "service", quantity: 2, unit_cents: 100 },
  ],
  publish: true,
});
assert.equal(published.workspace.projects[0].published[0].total_cents, 6197);
await workspace({
  revision: published.revision,
  action: "save_product",
  index: 0,
  item: { ...line, unit_cents: 2500 },
});
await workspace(
  {
    revision: published.revision,
    action: "save_quote",
    project: 0,
    lines: [line],
    publish: true,
  },
  key,
  409,
);
const saved = await workspace();
assert.equal(saved.workspace.catalog[0].unit_cents, 2500);
assert.deepEqual(
  saved.workspace.projects[0].published,
  published.workspace.projects[0].published,
);
assert.equal(
  (await workspace(undefined, randomBytes(32).toString("hex"))).workspace
    .projects.length,
  0,
);
await workspace(undefined, "", 401);
const questionnaire = randomBytes(32).toString("hex");
const active = await workspace({
  revision: saved.revision,
  action: "set_questionnaire",
  key: questionnaire,
});
const endpoint = new URL(`/api/questionnaires/${questionnaire}`, base);
assert.deepEqual(await (await fetch(endpoint)).json(), { open: true });
const inquiry = {
  id: randomUUID(),
  name: "客户问卷（虚构）",
  home: "新房待装修",
  systems: ["灯光", "窗帘"],
  budget: "待确认",
  notes: "示例：窗帘定时开启",
};
async function submit(body, status) {
  const response = await fetch(endpoint, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });
  assert.equal(response.status, status, await response.clone().text());
}
await submit({ ...inquiry, systems: ["无效选项"] }, 422);
await submit(inquiry, 200);
await submit(inquiry, 200);
const inbox = await workspace();
assert.equal(inbox.revision, active.revision + 1);
assert.equal(inbox.inbox.length, 1);
const other = await workspace(undefined, randomBytes(32).toString("hex"));
assert.deepEqual(other.inbox, []);
assert.equal(other.questionnaire_key, null);
const claimed = await workspace({
  revision: inbox.revision,
  action: "resolve_inquiry",
  id: inquiry.id,
  create_project: true,
});
assert.equal(claimed.inbox.length, 0);
assert.equal(claimed.workspace.projects[1].name, inquiry.name);
assert.match(claimed.workspace.projects[1].requirements, /窗帘定时开启/);
const closed = await workspace({
  revision: claimed.revision,
  action: "set_questionnaire",
  key: null,
});
assert.equal((await fetch(endpoint)).status, 404);
await submit({ ...inquiry, id: randomUUID() }, 404);
console.log(
  "Verified questionnaire submission, isolation, conversion and link closure.",
);

const wifi = {
  width_cm: 2000,
  height_cm: 1000,
  access_points: [{ x: 150, y: 500 }],
  walls: [
    { from: { x: 500, y: 0 }, to: { x: 500, y: 1000 }, material: "concrete" },
  ],
};
async function coverage(plan, status = 200) {
  const response = await fetch(new URL("/api/wifi/coverage", base), {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(plan),
  });
  assert.equal(response.status, status, await response.clone().text());
  if (status === 200) return response.json();
}
const floor = await coverage(wifi);
assert.equal(floor.levels.length, floor.size ** 2);
assert(
  floor.target_percent <
    (await coverage({ ...wifi, walls: [] })).target_percent,
);
await coverage({ ...wifi, width_cm: 0 }, 422);
await workspace({
  revision: closed.revision,
  action: "save_wifi",
  project: 0,
  plan: wifi,
});
assert.deepEqual((await workspace()).wifi, { 0: wifi });
assert.deepEqual(
  (await workspace(undefined, randomBytes(32).toString("hex"))).wifi,
  {},
);
console.log(
  "Verified Wi-Fi coverage, wall attenuation and project plan persistence.",
);

const artwork = {
  jpeg: readFileSync(new URL("./fixtures/floor.jpg", import.meta.url)).toString(
    "base64",
  ),
  pixel_width: 800,
  pixel_height: 400,
  from: { x: 250, y: 500 },
  to: { x: 750, y: 500 },
  distance_cm: 1000,
};
const saveFloor = {
  action: "save_wifi",
  project: 0,
  plan: wifi,
  floor: { kind: "set", floor: artwork },
};
const beforeFloor = await workspace();
await workspace(
  {
    ...saveFloor,
    revision: beforeFloor.revision,
    plan: { ...wifi, width_cm: 1800 },
  },
  key,
  422,
);
const withFloor = await workspace({
  ...saveFloor,
  revision: beforeFloor.revision,
});
assert.deepEqual(withFloor.wifi, { 0: wifi });
const artworkUrl = new URL("/api/wifi/floor/0", base);
const getArtwork = (token) =>
  fetch(artworkUrl, {
    headers: token ? { Authorization: `Bearer ${token}` } : {},
  });
assert.equal((await getArtwork()).status, 401);
assert.equal(
  await (await getArtwork(randomBytes(32).toString("hex"))).json(),
  null,
);
assert.deepEqual(await (await getArtwork(key)).json(), artwork);
await workspace(
  { ...saveFloor, revision: beforeFloor.revision, floor: { kind: "remove" } },
  key,
  409,
);
assert.deepEqual(await (await getArtwork(key)).json(), artwork);
const withoutFloor = await workspace({
  ...saveFloor,
  revision: withFloor.revision,
  floor: { kind: "remove" },
});
assert.deepEqual(withoutFloor.wifi, { 0: wifi });
assert.equal(await (await getArtwork(key)).json(), null);
console.log(
  "Verified calibrated floor persistence, read authorization and removal.",
);

if (main) {
  const response = await fetch(new URL("/api/workspace", main), {
    headers: { Authorization: `Bearer ${key}` },
  });
  assert.equal(response.status, 200);
  const mainWorkspace = await response.json();
  assert.equal(mainWorkspace.revision, 0);
  assert.deepEqual(mainWorkspace.workspace.projects, []);
  assert.deepEqual(mainWorkspace.wifi ?? {}, {});
  console.log("Verified preview writes are absent from main D1.");
}

console.log(
  "Verified D1 persistence, immutable published quote, stale revision rejection and workspace isolation.",
);
