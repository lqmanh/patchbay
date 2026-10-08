import { loadConfig } from "./core/config";
import { disableInterception, enableInterception } from "./core/interception";
import { requireRunOptions } from "./core/options";
import { listServers } from "./core/servers";
import { waitForReachable, type RunOptions } from "./core/wireproxy";

export async function runIntercept(action: string): Promise<number> {
  const config = await loadConfig();

  let options: RunOptions;
  try {
    options = requireRunOptions(config, await listServers());
  } catch (error) {
    console.error(`patchbay: ${error instanceof Error ? error.message : String(error)}`);
    return 1;
  }

  if (action === "off") {
    await disableInterception();
    console.log("interception off");
    return 0;
  }

  await waitForReachable(config.healthPort);
  await enableInterception(options);
  console.log("interception on");
  return 0;
}
