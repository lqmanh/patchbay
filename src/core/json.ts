import { ensureDirs } from "./paths";

export async function writeJson(path: string, value: unknown): Promise<void> {
  await ensureDirs();
  await Bun.write(path, `${JSON.stringify(value, null, 2)}\n`);
}
