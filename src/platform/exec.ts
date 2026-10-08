export function run(command: string, args: string[]): string {
  return Bun.spawnSync([command, ...args]).stdout.toString();
}

export function runChecked(command: string, args: string[]): string {
  const result = Bun.spawnSync([command, ...args]);
  if (result.exitCode !== 0) {
    const detail = result.stderr.toString().trim();
    throw new Error(
      `${command} exited with ${result.exitCode}${detail ? `: ${detail}` : ""}`,
    );
  }
  return result.stdout.toString();
}
