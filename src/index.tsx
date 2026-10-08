/** @jsxImportSource @opentui/solid */
import { createCliRenderer } from "@opentui/core";
import { render } from "@opentui/solid";
import { runIntercept } from "./cli";
import { INTERCEPT_ARGS } from "./core/interception";
import { App } from "./ui/app";

const [, , command, action] = process.argv;

if (command === INTERCEPT_ARGS[0]) {
  process.exit(await runIntercept(action ?? INTERCEPT_ARGS[1]));
}

const renderer = await createCliRenderer({ exitOnCtrlC: true });
await render(() => <App />, renderer);
