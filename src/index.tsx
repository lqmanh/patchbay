/** @jsxImportSource @opentui/solid */
import { createCliRenderer } from "@opentui/core";
import { KeymapProvider } from "@opentui/keymap/solid";
import { render } from "@opentui/solid";
import { runIntercept } from "./cli";
import { INTERCEPT_ARGS } from "./core/interception";
import { App } from "./ui/app";
import { createAppKeymap } from "./ui/keymap";

const [, , command, action] = process.argv;

if (command === INTERCEPT_ARGS[0]) {
  process.exit(await runIntercept(action ?? INTERCEPT_ARGS[1]));
}

const renderer = await createCliRenderer({ exitOnCtrlC: true, useMouse: true });
const keymap = createAppKeymap(renderer);
await render(
  () => (
    <KeymapProvider keymap={keymap}>
      <App />
    </KeymapProvider>
  ),
  renderer,
);
