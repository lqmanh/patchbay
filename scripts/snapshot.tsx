/** @jsxImportSource @opentui/solid */
import { createTestRenderer } from "@opentui/core/testing";
import { KeymapProvider } from "@opentui/keymap/solid";
import { render } from "@opentui/solid";
import { App } from "../src/ui/app";
import { createAppKeymap } from "../src/ui/keymap";

const setup = await createTestRenderer({
  width: Number(process.argv[2] ?? 100),
  height: Number(process.argv[3] ?? 30),
});

try {
  const keymap = createAppKeymap(setup.renderer);
  await render(
    () => (
      <KeymapProvider keymap={keymap}>
        <App />
      </KeymapProvider>
    ),
    setup.renderer,
  );
  await setup.waitForFrame((frame) => frame.includes("domain rules"));
  await setup.waitForFrame((frame) => frame.includes("(active)"));
  const keys = (process.argv[4] ?? "").split(",").filter(Boolean);
  if (keys.length > 0) {
    await setup.mockInput.pressKeys(keys);
    await setup.renderOnce();
    await setup.flush();
  }
  await setup.waitForVisualIdle();

  if (process.argv[5] === "spans") {
    const frame = setup.captureSpans();
    for (const [index, line] of frame.lines.entries()) {
      const text = line.spans.map((span) => span.text).join("");
      if (!text.includes("intercept")) continue;
      console.log(`line ${index + 1}: ${JSON.stringify(text)}`);
      for (const span of line.spans) {
        if (span.text)
          console.log(`   ${JSON.stringify(span.text)} fg=${span.fg.toString()}`);
      }
    }
  } else {
    const lines = setup.captureCharFrame().split("\n");
    lines.forEach((line, index) => {
      console.log(`${String(index + 1).padStart(2, " ")}|${line.replace(/\s+$/, "")}`);
    });
  }
} finally {
  setup.renderer.destroy();
}
