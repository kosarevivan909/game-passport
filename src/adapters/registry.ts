import { UnsupportedAdapter } from "./UnsupportedAdapter";
import { Cs2Adapter } from "./Cs2Adapter";
import { Cs2PreflightAdapter } from "./Cs2PreflightAdapter";
import { DisplayAdapter } from "./DisplayAdapter";
import { NvidiaAdapter } from "./NvidiaAdapter";
import { MouseAdapter } from "./MouseAdapter";
import { PubgAdapter } from "./PubgAdapter";
import { PubgPreflightAdapter } from "./PubgPreflightAdapter";

const cs2 = new Cs2Adapter();
const display = new DisplayAdapter();
const nvidia = new NvidiaAdapter();
const preflight = new Cs2PreflightAdapter();
const mouse = new MouseAdapter();
const pubg = new PubgAdapter();
const pubgPreflight = new PubgPreflightAdapter();

export const adapterRegistry = [
  display,
  nvidia,
  cs2,
  new UnsupportedAdapter("game.dota2", "Dota 2 Settings", "dota2"),
  pubg,
  new UnsupportedAdapter("windows", "Windows"),
  new UnsupportedAdapter("audio", "Audio"),
  mouse
];

export const captureAdapterRegistry = [cs2, pubg, display, nvidia, mouse];
export const applyAdapterRegistry = [preflight, pubgPreflight, display, nvidia, mouse, cs2, pubg];
export const restoreAdapterRegistry = [preflight, pubgPreflight, cs2, pubg, mouse, nvidia, display];
