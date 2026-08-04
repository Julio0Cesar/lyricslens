import { useState } from "react";
import type { Dicionario } from "../i18n";

/**
 * Nomes que o Hyprland usa e que não são simplesmente a tecla em maiúscula.
 */
const NOMES: Record<string, string> = {
  " ": "space",
  Escape: "escape",
  Enter: "return",
  Tab: "tab",
  Backspace: "backspace",
  Delete: "delete",
  ArrowUp: "up",
  ArrowDown: "down",
  ArrowLeft: "left",
  ArrowRight: "right",
  Home: "home",
  End: "end",
  PageUp: "prior",
  PageDown: "next",
  ",": "comma",
  ".": "period",
  "/": "slash",
  ";": "semicolon",
  "'": "apostrophe",
  "[": "bracketleft",
  "]": "bracketright",
  "-": "minus",
  "=": "equal",
  "\\": "backslash",
  "`": "grave",
};

const SO_MODIFICADOR = new Set(["Control", "Alt", "Shift", "Meta", "Super"]);

/** Monta a notação do compositor: `SUPER SHIFT, L`. */
function traduzir(e: React.KeyboardEvent): string | null {
  if (SO_MODIFICADOR.has(e.key)) return null;

  const mods: string[] = [];
  if (e.metaKey) mods.push("SUPER");
  if (e.ctrlKey) mods.push("CTRL");
  if (e.altKey) mods.push("ALT");
  if (e.shiftKey) mods.push("SHIFT");

  const tecla = NOMES[e.key] ?? (e.key.length === 1 ? e.key.toUpperCase() : e.key.toLowerCase());
  return `${mods.join(" ")}, ${tecla}`;
}

export default function HotkeyCapture({
  value,
  onChange,
  t,
}: {
  value: string;
  onChange: (v: string) => void;
  t: Dicionario;
}) {
  const [capturando, setCapturando] = useState(false);

  return (
    <>
      <button
        type="button"
        onClick={() => setCapturando((c) => !c)}
        onBlur={() => setCapturando(false)}
        onKeyDown={(e) => {
          if (!capturando) return;
          e.preventDefault();
          const combo = traduzir(e);
          if (!combo) return; // ainda segurando só os modificadores
          onChange(combo);
          setCapturando(false);
        }}
        className={`w-44 rounded border px-2 py-1 text-center font-mono text-[11px] ${
          capturando
            ? "border-emerald-400/60 bg-emerald-400/10 text-emerald-200"
            : "border-white/15 bg-white/6 text-white/80 hover:bg-white/12"
        }`}
      >
        {capturando ? t["hotkey.capturing"] : value || t["hotkey.none"]}
      </button>
      {value && !capturando && (
        <button
          type="button"
          onClick={() => onChange("")}
          className="rounded border border-white/15 px-1.5 py-1 text-[11px] text-white/45 hover:bg-white/10"
        >
          ✕
        </button>
      )}
    </>
  );
}
