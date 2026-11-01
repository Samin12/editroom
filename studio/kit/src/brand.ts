// Written by SageCut from sagecut/brand.yaml. Do not edit by hand: edit the brand file, then run "sagecut brand set sagecut/brand.yaml".
// Needs @remotion/fonts in the project: npx remotion add @remotion/fonts
import { staticFile } from 'remotion';
import { loadFont } from '@remotion/fonts';

export const BRAND_NAME = "borumi-velour-dark";

export const colors = {
  background: "#121212",
  text: "#F4F1EC",
  primary: "#E2774E",
  accent: ["#C9643F","#7FC9AE","#E0B04F"] as readonly string[],
  allowed: ["#1E1D1B","#2A2825","#33302B","#34322E","#45413A","#0C0C0B","#D9D4CC","#A39C91","#6E685F","#3A241A","#F6F1E9","#9BDCC3","#1F3A31","#46B482","#E5564A","#FFFFFF"] as readonly string[],
} as const;

export const fonts = {
  heading: "Inter",
  body: "Inter"
} as const;

export const type = {"headline_px":64,"body_px":44,"caption_px":32} as const;
export const motion = {"style":"restrained","designed":["hard_cut"]} as const;
export const copyRules = {"case":"sentence","avoid":[],"grammar":"casual"} as const;
export const format = "16:9";
export const logos: readonly string[] = [];

const FONT_FACES = [
  { family: "Inter", weight: "700", url: staticFile("fonts/InterVariable.woff2") },
];

// Load every brand font. Call it once and wait before the first frame renders:
//   const handle = delayRender(); loadBrandFonts().then(() => continueRender(handle));
export const loadBrandFonts = () => Promise.all(FONT_FACES.map((f) => loadFont(f)));
