/**
 * Course roadmap for Samin's "YouTube course" video (2026-10-28). Additive and opt-in: nothing else imports it.
 * Shared by every section job (M-ROADMAP in the proof pack) so the map looks identical each time it appears.
 *
 * What he says on camera (0:02:54-0:04:09): a FOUNDATION first (best practices, viewer avatar, set-up), then eight
 * stages (ideation, research, packaging, script, film, edit, repurpose, publish), and "at the very end, we're gonna put
 * an AI agent team on top of everything so it runs on autopilot". He calls packaging "step two" later, so the map shows
 * stage NAMES, never numbers.
 *
 *   <CourseRoadmap current={3} />                 // packaging is the current stage; ideation + research done
 *   <CourseRoadmap current={0} foundation="current" />   // the foundation section
 *   <CourseRoadmap current={9} agents="current" />       // everything done, the agent team lights on top
 * Build-up (scene 2): animate `current` yourself (e.g. from the word timings) or use `from` + `delay` for one sweep.
 * Authored for the 1920x1080 stage (BEHIND jobs, scaled into the panel). Default box: x 150, y 300, width 1620.
 */
import React from "react";
import { useCurrentFrame } from "remotion";
import { C, prog, mix, EASE_IN_OUT } from "./theme";
import { StepTracker } from "./components";

export const COURSE_STAGES = ["Ideation", "Research", "Packaging", "Script", "Film", "Edit", "Repurpose", "Publish"];

type Lit = "off" | "current" | "done";

const Slab: React.FC<{ label: string; sub?: string; state: Lit; x: number; y: number; w: number; h: number; delay: number; out?: number }> = ({
  label, sub, state, x, y, w, h, delay, out,
}) => {
  const f = useCurrentFrame();
  const p = prog(f, delay, 18);
  const o = out != null ? 1 - prog(f, out, 12, EASE_IN_OUT) : 1;
  const cur = state === "current";
  const done = state === "done";
  const pulse = cur ? 0.5 + 0.5 * Math.sin((f / 30) * Math.PI) : 0;
  return (
    <div
      style={{
        position: "absolute", left: x, top: y + (1 - p) * 18, width: w, height: h, opacity: p * o, borderRadius: 22,
        boxSizing: "border-box", display: "flex", alignItems: "center", justifyContent: "center", gap: 22,
        background: cur ? C.accent : done ? C.dark : C.paper,
        border: cur ? "none" : `3px solid ${done ? C.darkLine : C.line}`,
        boxShadow: cur ? `0 14px ${30 + pulse * 18}px -10px ${C.accent}` : undefined,
        fontFamily: "Inter, system-ui, sans-serif",
      }}
    >
      {done ? (
        <svg width={34} height={34} viewBox="0 0 24 24" style={{ marginRight: -6 }}>
          <path d="M5 12.5 10 17.5 19 7.5" fill="none" stroke={C.green} strokeWidth={3.4} strokeLinecap="round" strokeLinejoin="round" />
        </svg>
      ) : null}
      <span style={{ fontSize: 40, fontWeight: 800, letterSpacing: "0.12em", color: cur ? C.white : done ? C.inkSoft : C.faint }}>{label}</span>
      {sub ? <span style={{ fontSize: 30, fontWeight: 600, color: cur ? C.white : C.muted, opacity: 0.9 }}>{sub}</span> : null}
    </div>
  );
};

export const CourseRoadmap: React.FC<{
  /** 0 = no stage yet (foundation phase), 1..8 = that stage is current (earlier ones done), 9 = all eight done */
  current: number;
  from?: number;
  x?: number;
  y?: number;
  width?: number;
  delay?: number;
  dur?: number;
  foundation?: Lit;
  agents?: Lit;
  /** show the "AI agent team on top" bar at all (default true) */
  showAgents?: boolean;
  /** override the slab sub-labels to match his words in that stretch ("" hides them); defaults unchanged */
  foundationSub?: string;
  agentsSub?: string;
  out?: number;
}> = ({ current, from, x = 150, y = 300, width = 1620, delay = 0, dur = 24, foundation, agents, showAgents = true, out,
         foundationSub = "viewer, set-up, your system", agentsSub = "runs it on autopilot" }) => {
  const f = useCurrentFrame();
  const fState: Lit = foundation ?? (current >= 1 ? "done" : "current");
  const aState: Lit = agents ?? (current >= 9 ? "current" : "off");
  const trackCurrent = Math.min(current, 9);
  const o = out != null ? 1 - prog(f, out, 12, EASE_IN_OUT) : 1;
  const node = 72; // labels = 0.4 x node = 28.8 stage px (the 28 px label floor)
  const pad = 60;
  return (
    <div style={{ position: "absolute", left: x, top: y, width, height: 520, opacity: o }}>
      {showAgents ? (
        <Slab label="AI AGENT TEAM" sub={agentsSub || undefined} state={aState} x={0} y={0} w={width} h={96} delay={delay + 4} />
      ) : null}
      <div style={{ position: "absolute", left: 0, top: 128, width, height: 228, overflow: "hidden" }}>
        <StepTracker
          current={trackCurrent > 8 ? 9 : trackCurrent}
          from={from}
          x={pad}
          y={40}
          width={width - pad * 2}
          delay={delay}
          dur={dur}
          tone="dark"
          labels
          steps={COURSE_STAGES}
          node={node}
          revealStagger={2}
          numbers={false}
        />
      </div>
      <Slab label="FOUNDATION" sub={foundationSub || undefined} state={fState} x={0} y={372} w={width} h={96} delay={delay + 2} />
      <div style={{ position: "absolute", left: 0, top: 356, width, height: 4, opacity: mix(prog(f, delay + 6, 16), 0, 0.6), background: C.line }} />
    </div>
  );
};
