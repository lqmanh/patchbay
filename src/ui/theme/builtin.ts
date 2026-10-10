import aura from "./assets/aura.json";
import ayu from "./assets/ayu.json";
import carbonfox from "./assets/carbonfox.json";
import catppuccinFrappe from "./assets/catppuccin-frappe.json";
import catppuccinMacchiato from "./assets/catppuccin-macchiato.json";
import catppuccin from "./assets/catppuccin.json";
import cobalt2 from "./assets/cobalt2.json";
import cursor from "./assets/cursor.json";
import dracula from "./assets/dracula.json";
import everforest from "./assets/everforest.json";
import flexoki from "./assets/flexoki.json";
import github from "./assets/github.json";
import gruvbox from "./assets/gruvbox.json";
import kanagawa from "./assets/kanagawa.json";
import lucentOrng from "./assets/lucent-orng.json";
import material from "./assets/material.json";
import matrix from "./assets/matrix.json";
import mercury from "./assets/mercury.json";
import monokai from "./assets/monokai.json";
import nightowl from "./assets/nightowl.json";
import nord from "./assets/nord.json";
import oneDark from "./assets/one-dark.json";
import opencode from "./assets/opencode.json";
import orng from "./assets/orng.json";
import osakaJade from "./assets/osaka-jade.json";
import palenight from "./assets/palenight.json";
import rosepine from "./assets/rosepine.json";
import solarized from "./assets/solarized.json";
import synthwave84 from "./assets/synthwave84.json";
import tokyonight from "./assets/tokyonight.json";
import vercel from "./assets/vercel.json";
import vesper from "./assets/vesper.json";
import zenburn from "./assets/zenburn.json";
import { type ThemeJson } from "./loader";

export const BUILTIN_THEMES: Record<string, ThemeJson> = {
  aura,
  ayu,
  carbonfox,
  catppuccin,
  "catppuccin-frappe": catppuccinFrappe,
  "catppuccin-macchiato": catppuccinMacchiato,
  cobalt2,
  cursor,
  dracula,
  everforest,
  flexoki,
  github,
  gruvbox,
  kanagawa,
  "lucent-orng": lucentOrng,
  material,
  matrix,
  mercury,
  monokai,
  nightowl,
  nord,
  "one-dark": oneDark,
  opencode,
  orng,
  "osaka-jade": osakaJade,
  palenight,
  rosepine,
  solarized,
  synthwave84,
  tokyonight,
  vercel,
  vesper,
  zenburn,
};

export const THEME_NAMES = ["system", ...Object.keys(BUILTIN_THEMES).sort()];
