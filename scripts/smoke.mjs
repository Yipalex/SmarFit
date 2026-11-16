import assert from "node:assert/strict";
import { randomBytes } from "node:crypto";

const base = process.argv[2];
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
console.log(
  "Verified D1 persistence, immutable published quote, stale revision rejection and workspace isolation.",
);
