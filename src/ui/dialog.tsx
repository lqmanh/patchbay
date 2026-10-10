/** @jsxImportSource @opentui/solid */
import { RGBA } from "@opentui/core";
import type { JSX } from "solid-js";

const BACKDROP = RGBA.fromValues(0, 0, 0, 0.5);

export function Dialog(props: { onClose: () => void; children: JSX.Element }) {
  return (
    <box
      position="absolute"
      left={0}
      top={0}
      width="100%"
      height="100%"
      alignItems="center"
      justifyContent="center"
    >
      <box
        position="absolute"
        left={0}
        top={0}
        width="100%"
        height="100%"
        backgroundColor={BACKDROP}
        onMouseDown={() => props.onClose()}
      />
      {props.children}
    </box>
  );
}
