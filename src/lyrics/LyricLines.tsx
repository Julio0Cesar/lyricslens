import { useMemo } from "react";
import { AnimatePresence, motion } from "motion/react";
import type { Settings } from "../settings/useSettings";
import type { LyricLine } from "./useLyrics";

/**
 * A varredura acende **palavra por palavra**, não caractere por caractere.
 *
 * Um gradiente contínuo seria mais suave, mas o recorte de gradiente vale para
 * a caixa inteira: numa linha que quebra em duas, as duas acenderiam ao mesmo
 * tempo. Palavra é a maior unidade que sobrevive à quebra de linha — e cada
 * uma acende com transição própria, então o resultado é contínuo aos olhos.
 */
function Karaoke({
  text,
  progress,
  settings,
}: {
  text: string;
  progress: number;
  settings: Settings;
}) {
  const pedacos = useMemo(() => {
    // Mantém os espaços como pedaços próprios para não colapsar o texto.
    const partes = text.split(/(\s+)/).filter(Boolean);
    let percorrido = 0;
    return partes.map((parte) => {
      const inicio = percorrido / Math.max(1, text.length);
      percorrido += parte.length;
      return { parte, inicio };
    });
  }, [text]);

  if (!settings.karaoke) {
    return <span style={{ color: settings.textColor }}>{text}</span>;
  }

  return (
    <>
      {pedacos.map(({ parte, inicio }, i) => (
        <span
          key={i}
          style={{
            color: inicio < progress ? settings.textColor : settings.dimColor,
            transition: "color 260ms ease-out",
          }}
        >
          {parte}
        </span>
      ))}
    </>
  );
}

/** O intervalo instrumental, que no LRC é uma linha sem texto. */
function Instrumental({ settings }: { settings: Settings }) {
  return (
    <div className="flex items-center gap-1.5" style={{ height: settings.fontSize * 1.3 }}>
      {[0, 1, 2].map((i) => (
        <motion.span
          key={i}
          className="block rounded-full"
          style={{
            width: settings.fontSize * 0.18,
            height: settings.fontSize * 0.18,
            background: settings.dimColor,
          }}
          animate={{ opacity: [0.25, 1, 0.25] }}
          transition={{ duration: 1.6, repeat: Infinity, delay: i * 0.22, ease: "easeInOut" }}
        />
      ))}
    </div>
  );
}

/**
 * As linhas sobem juntas conforme a música anda. Cada uma entra por baixo
 * desfocada, ganha foco ao virar a atual, e sai por cima — o movimento é o que
 * dá a sensação de que a letra está passando, e não trocando de conteúdo.
 */
export default function LyricLines({
  lines,
  index,
  progress,
  settings,
}: {
  lines: LyricLine[];
  index: number;
  progress: number;
  settings: Settings;
}) {
  const janela: number[] = [];
  if (settings.showContextLines && index - 1 >= 0) janela.push(index - 1);
  if (index >= 0) janela.push(index);
  if (settings.showContextLines && index + 1 < lines.length) janela.push(index + 1);

  const alinhamento = settings.textAlign === "center" ? "items-center" : "items-start";

  return (
    <div className={`flex flex-col gap-1 ${alinhamento}`}>
      <AnimatePresence initial={false} mode="popLayout">
        {janela.map((i) => {
          const atual = i === index;
          const texto = lines[i]?.text ?? "";

          return (
            <motion.div
              key={i}
              // `position` evita que a escala das vizinhas distorça o texto
              // durante o reposicionamento.
              layout="position"
              initial={{ opacity: 0, y: 26, filter: "blur(7px)" }}
              animate={{
                opacity: atual ? 1 : 0.4,
                y: 0,
                filter: "blur(0px)",
                scale: atual ? 1 : 0.92,
              }}
              exit={{ opacity: 0, y: -26, filter: "blur(7px)" }}
              transition={{ type: "spring", stiffness: 280, damping: 32, mass: 0.7 }}
              style={{
                transformOrigin: settings.textAlign === "center" ? "center" : "left center",
                fontSize: atual ? settings.fontSize : Math.round(settings.fontSize * 0.62),
                fontWeight: atual ? settings.fontWeight : 500,
                color: settings.dimColor,
                lineHeight: 1.25,
              }}
              className="max-w-full"
            >
              {atual && !texto ? (
                <Instrumental settings={settings} />
              ) : atual ? (
                <Karaoke text={texto} progress={progress} settings={settings} />
              ) : (
                texto
              )}
            </motion.div>
          );
        })}
      </AnimatePresence>
    </div>
  );
}
