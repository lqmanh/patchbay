import solidPlugin from "@opentui/solid/bun-plugin";
import { readdir, rm } from "node:fs/promises";

const target = process.arch === "arm64" ? "bun-darwin-arm64" : "bun-darwin-x64";

// Bun 1.4.x leaves a hidden .bun-build staging file behind on every compile.
async function removeTempBuilds(): Promise<void> {
  const entries = await readdir(".", { withFileTypes: true });
  await Promise.all(
    entries
      .filter((entry) => entry.isFile() && entry.name.endsWith(".bun-build"))
      .map((entry) => rm(entry.name)),
  );
}

const result = await Bun.build({
  entrypoints: ["./src/index.tsx"],
  target: "bun",
  outdir: "./dist",
  plugins: [solidPlugin],
  compile: {
    target,
    outfile: "patchbay",
    // The standalone binary must not autoload bunfig.toml: its dev preload
    // doesn't exist at runtime and breaks startup.
    autoloadBunfig: false,
  },
});

await removeTempBuilds();

if (!result.success) {
  for (const log of result.logs) console.error(log);
  process.exit(1);
}

console.log(`built dist/patchbay for ${target}`);
