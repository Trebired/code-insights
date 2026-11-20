import { button, canvas_panel, input } from "@trebired/frontend/react";
import type { ReactNode } from "react";

import { copyFor } from "#0no5pre0uzbk";
import type { CodeMapLevel } from "#dzbbbwiozeyj";

type CodeMapProps = {
  dataUrl: string;
  extendGroup?: string;
  extendId?: string;
  fullscreen?: boolean;
  hrefTemplate?: string;
  id?: string;
  lang?: string;
  level?: CodeMapLevel;
  title?: ReactNode;
  toolbar?: ReactNode;
};

function levelButton(level: CodeMapLevel, current: CodeMapLevel, label: string) {
  return button({
      "aria-pressed": level === current ? "true" : "false",
      children: label,
      "data-code-map-switch": level,
      size: "sm",
      type: "button",
      variant: "primary",
    } as any);
}

function code_map(props: CodeMapProps) {
  const copy = copyFor(props.lang);
  const level: CodeMapLevel = props.level === "function" ? "function" : "file";
  const toolbar = (
    <>
    {levelButton("file", level, copy("files"))}
    {levelButton("function", level, copy("functions"))}
    {props.toolbar}
    </>
  );
  const content = (
    <div
    className="tbf-column tbf-gap-sm"
    data-code-map-root=""
    data-code-map-url={props.dataUrl}
    data-code-map-level={level}
    data-code-map-href={props.hrefTemplate || ""}
    data-code-map-lang={props.lang || "en"}
    id={props.id}
    >
    {input({
          "aria-label": copy("filter"),
          autoComplete: "off",
          "data-code-map-filter": "",
          placeholder: copy("filter"),
          size: "sm",
          type: "search",
        } as any)}
    <svg
    data-code-map-canvas="" role="img" aria-label={copy("codeMap")}
    style={{ display: "block", height: "32rem", maxHeight: "70vh", touchAction: "none", width: "100%" }} />
    <div className="tbf-text-muted tbf-text-sm" data-code-map-status="">{copy("loading")}</div>
    <div className="tbf-text-muted tbf-text-xs">{copy("hint")}</div>
    </div>
  );
  return canvas_panel({
      content,
      contentClassName: "tbf-column tbf-gap-sm",
      extendGroup: props.extendGroup,
      extendId: props.extendId,
      fullscreen: props.fullscreen === true,
      lang: props.lang,
      title: props.title ?? copy("codeMap"),
      toolbarContent: toolbar,
    } as any);
}

export { code_map };
export type { CodeMapProps };
