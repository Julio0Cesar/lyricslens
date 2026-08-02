import type { ReactNode } from "react";

export function Section({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section className="flex flex-col gap-3">
      <h2 className="text-[11px] font-semibold tracking-[0.12em] text-white/40 uppercase">
        {title}
      </h2>
      <div className="flex flex-col gap-3">{children}</div>
    </section>
  );
}

export function Row({ label, hint, children }: { label: string; hint?: string; children: ReactNode }) {
  return (
    <label className="flex items-center justify-between gap-4">
      <span className="flex min-w-0 flex-col">
        <span className="text-sm text-white/85">{label}</span>
        {hint && <span className="text-[11px] text-white/35">{hint}</span>}
      </span>
      <span className="flex shrink-0 items-center gap-2">{children}</span>
    </label>
  );
}

export function Slider({
  value,
  min,
  max,
  step = 1,
  suffix,
  onChange,
}: {
  value: number;
  min: number;
  max: number;
  step?: number;
  suffix?: string;
  onChange: (v: number) => void;
}) {
  return (
    <>
      <input
        type="range"
        min={min}
        max={max}
        step={step}
        value={value}
        onChange={(e) => onChange(Number(e.currentTarget.value))}
        className="w-36 accent-white/80"
      />
      <span className="w-14 text-right font-mono text-[11px] text-white/50 tabular-nums">
        {step < 1 ? value.toFixed(2) : value}
        {suffix}
      </span>
    </>
  );
}

export function Toggle({ value, onChange }: { value: boolean; onChange: (v: boolean) => void }) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={value}
      onClick={() => onChange(!value)}
      className={`h-5 w-9 rounded-full p-0.5 transition-colors ${
        value ? "bg-emerald-400/80" : "bg-white/15"
      }`}
    >
      <span
        className={`block h-4 w-4 rounded-full bg-white transition-transform ${
          value ? "translate-x-4" : "translate-x-0"
        }`}
      />
    </button>
  );
}

export function ColorPicker({ value, onChange }: { value: string; onChange: (v: string) => void }) {
  // O input de cor do WebKit não entende alfa; o hex de 8 dígitos é preservado
  // no texto ao lado, que aceita a forma completa.
  return (
    <>
      <input
        type="color"
        value={value.slice(0, 7)}
        onChange={(e) => onChange(e.currentTarget.value + value.slice(7))}
        className="h-6 w-8 cursor-pointer rounded border border-white/15 bg-transparent"
      />
      <input
        type="text"
        value={value}
        onChange={(e) => onChange(e.currentTarget.value)}
        spellCheck={false}
        className="w-24 rounded border border-white/15 bg-white/5 px-1.5 py-0.5 font-mono text-[11px] text-white/70"
      />
    </>
  );
}

export function Select<T extends string>({
  value,
  options,
  onChange,
}: {
  value: T;
  options: { value: T; label: string }[];
  onChange: (v: T) => void;
}) {
  // Sem `appearance-none` o WebKit desenha o controle nativo — fundo claro do
  // sistema com o texto claro do tema, ou seja, ilegível.
  return (
    <div className="relative">
      <select
        value={value}
        onChange={(e) => onChange(e.currentTarget.value as T)}
        className="w-44 appearance-none rounded border border-white/15 bg-neutral-800 py-1 pr-7 pl-2 text-[12px] text-white/85"
      >
        {options.map((o) => (
          <option key={o.value} value={o.value} className="bg-neutral-800 text-white">
            {o.label}
          </option>
        ))}
      </select>
      <span className="pointer-events-none absolute top-1/2 right-2 -translate-y-1/2 text-[9px] text-white/40">
        ▼
      </span>
    </div>
  );
}
