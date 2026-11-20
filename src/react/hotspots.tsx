import { Fragment } from "react";
import { actionTrigger, card, Title, tooltipAttrs } from "@trebired/frontend/react";

import { copyFor, countFor } from "#0no5pre0uzbk";
import type { Hotspot } from "#dzbbbwiozeyj";

type HotspotListProps = {
  hotspots: Hotspot[];
  hrefFor?: (path: string) => string;
  lang?: string;
  limit?: number;
  title?: string;
};

const TRACK_STYLE = {
  background: "var(--border-surface-1)",
  borderRadius: "999px",
  display: "block",
  flex: "1 1 auto",
  height: "6px",
  minWidth: "3rem",
  overflow: "hidden",
};

function hotspotRow(hotspot: Hotspot, props: HotspotListProps) {
  const count = countFor(props.lang);
  const row = (
    <div className="tbfm-card-03" data-code-insights-hotspot="">
    <span className="tbfm-card-03__lead tbfm-card-03__text" {...tooltipAttrs(hotspot.path, { overflow: true })}>{hotspot.path}</span>
    <span className="tbfm-card-03__main">
    <span
    aria-hidden="true"
    style={TRACK_STYLE}>
    <span
    style={{ background: "var(--text-color-muted)", display: "block", height: "100%", width: `${Math.max(2, Math.min(100, hotspot.score))}%` }} />
    </span>
    </span>
    <span className="tbfm-card-03__end tbf-text-muted tbf-text-sm">
    {count(hotspot.commits, "commits")} · {count(hotspot.lines, "lines")}
    </span>
    </div>
  );
  const href = props.hrefFor ? props.hrefFor(hotspot.path) : "";
  return href ? actionTrigger(row, { href }) : row;
}

function hotspot_list(props: HotspotListProps) {
  const copy = copyFor(props.lang);
  const hotspots = (Array.isArray(props.hotspots) ? props.hotspots : []).slice(0, Math.max(1, props.limit || 10));
  return card({
      className: "tbf-column tbf-gap-sm",
      children: (
        <>
        <div className="tbf-column tbf-gap-xs">
        <Title>{props.title || copy("hotspots")}</Title>
        <p className="tbf-text-muted">{copy("hotspotsDescription")}</p>
        </div>
        {hotspots.length ? (
            <div className="tbf-column tbf-divided">
            {hotspots.map((hotspot) => <Fragment key={hotspot.path}>{hotspotRow(hotspot, props)}</Fragment>)}
            </div>
          ) : (
            <span className="tbf-text-muted">{copy("hotspotsEmpty")}</span>
        )}
        </>
      ),
  });
}

export { hotspot_list };
export type { HotspotListProps };
