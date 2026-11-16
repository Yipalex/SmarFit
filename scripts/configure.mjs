import { readFileSync, writeFileSync } from "node:fs";

const base = new URL("../worker/wrangler.jsonc", import.meta.url);
const config = JSON.parse(readFileSync(base, "utf8"));
const local = process.argv.includes("--local");
const preview = process.argv.includes("--preview");
const id = local
  ? "00000000-0000-4000-8000-000000000001"
  : preview
    ? process.env.SMARFIT_D1_PREVIEW
    : process.env.SMARFIT_D1_MAIN;

if (!id || !/^[\da-f]{8}(?:-[\da-f]{4}){3}-[\da-f]{12}$/i.test(id)) {
  throw new Error("Set the D1 database ID for this deployment environment.");
}
if (!local && process.env.SMARFIT_D1_MAIN === process.env.SMARFIT_D1_PREVIEW) {
  throw new Error("Main and preview must use different D1 databases.");
}

const binding = {
  binding: "DB",
  database_id: id,
  database_name: local
    ? "smarfit-local"
    : preview
      ? "smarfit-preview"
      : "smarfit-main",
  migrations_dir: "../migrations",
};
config.d1_databases = [binding];
if (preview) config.previews.d1_databases = [binding];
writeFileSync(
  new URL("wrangler.generated.json", base),
  JSON.stringify(config, null, 2),
);
