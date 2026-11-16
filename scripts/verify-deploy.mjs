import assert from "node:assert/strict";
import { setTimeout as delay } from "node:timers/promises";

const base = process.argv[2];
for (let attempt = 0; ; attempt++) {
  try {
    const options = { signal: AbortSignal.timeout(8000) };
    const [home, health, version] = await Promise.all(
      ["/", "/api/health", "/api/version"].map((path) =>
        fetch(new URL(path, base), options),
      ),
    );
    assert(home.ok && health.ok && version.ok);
    assert.match(await home.text(), /SmarFit/);
    assert.equal((await health.json()).status, "ok");
    const build = await version.json();
    assert.equal(build.marker, "ximo");
    if (process.env.GITHUB_SHA)
      assert.equal(build.commit, process.env.GITHUB_SHA);
    console.log(`Verified ${base} at ${build.commit}`);
    break;
  } catch (error) {
    if (attempt === 9) throw error;
    await delay(3000);
  }
}
