import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const worker = fileURLToPath(new URL("../worker/", import.meta.url));
const commit =
  process.env.GITHUB_SHA ??
  execFileSync("git", ["rev-parse", "HEAD"], {
    cwd: worker,
    encoding: "utf8",
  }).trim();
execFileSync("worker-build", ["--release"], {
  cwd: worker,
  env: { ...process.env, SMARFIT_COMMIT: commit },
  stdio: "inherit",
});
