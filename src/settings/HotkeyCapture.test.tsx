// @vitest-environment jsdom
/**
 * A captura de atalho.
 *
 * A tradução de um evento de teclado para a notação do compositor é lógica
 * pura, mas mora dentro do componente e só é alcançável por ele — a #9 a lista
 * entre as funções sem nenhum teste. O que ela produz vai direto para o
 * `hyprctl`: errar aqui é o compositor recusar, ou pior, registrar a
 * combinação errada.
 */
import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import HotkeyCapture from "./HotkeyCapture";
import { textos } from "../i18n";

const t = textos("pt-BR");

function montar(valor = "") {
  const onChange = vi.fn();
  render(<HotkeyCapture value={valor} onChange={onChange} t={t} />);
  return { onChange, usuario: userEvent.setup() };
}

const botao = () => screen.getAllByRole("button")[0];

describe("estado do botão", () => {
  it("sem atalho, convida a definir um", () => {
    montar();
    expect(botao()).toHaveTextContent("nenhum");
  });

  it("com atalho, mostra a combinação", () => {
    montar("SUPER, L");
    expect(botao()).toHaveTextContent("SUPER, L");
  });

  it("ao clicar, entra em modo de captura", async () => {
    const { usuario } = montar();
    await usuario.click(botao());
    expect(botao()).toHaveTextContent("pressione…");
  });

  it("perder o foco cancela a captura", async () => {
    const { usuario } = montar();
    await usuario.click(botao());
    await usuario.tab();
    expect(botao()).toHaveTextContent("nenhum");
  });
});

describe("tradução para a notação do compositor", () => {
  // Tecla de um caractere vai em maiúscula: é como o compositor a nomeia, e
  // `SUPER, l` e `SUPER, L` seriam binds diferentes para ele.
  it("modificador mais tecla, com a tecla em maiúscula", async () => {
    const { onChange, usuario } = montar();
    await usuario.click(botao());
    await usuario.keyboard("{Meta>}l{/Meta}");
    expect(onChange).toHaveBeenCalledWith("SUPER, L");
  });

  it("junta vários modificadores", async () => {
    const { onChange, usuario } = montar();
    await usuario.click(botao());
    await usuario.keyboard("{Control>}{Shift>}k{/Shift}{/Control}");
    expect(onChange).toHaveBeenCalledWith("CTRL SHIFT, K");
  });

  // A ordem é fixa, não a de digitação: o mesmo atalho precisa produzir a
  // mesma cadeia sempre, senão o `unbind` da troca não casa com o que foi
  // registrado e os binds se empilham no compositor.
  it("a ordem dos modificadores não depende da ordem de digitação", async () => {
    const { onChange, usuario } = montar();
    await usuario.click(botao());
    await usuario.keyboard("{Shift>}{Control>}k{/Control}{/Shift}");
    expect(onChange).toHaveBeenCalledWith("CTRL SHIFT, K");
  });

  /// Teclas sem caractere próprio precisam do nome que o compositor conhece.
  it("usa o nome que o compositor entende para teclas especiais", async () => {
    const { onChange, usuario } = montar();
    await usuario.click(botao());
    await usuario.keyboard("{Alt>}{ArrowUp}{/Alt}");
    expect(onChange).toHaveBeenCalledWith("ALT, up");
  });

  /// Segurar só modificador não é atalho — e se fosse, capturaria sozinho no
  /// instante em que o usuário apertasse a primeira tecla da combinação.
  it("modificador sozinho não fecha a captura", async () => {
    const { onChange, usuario } = montar();
    await usuario.click(botao());
    await usuario.keyboard("{Control>}");
    expect(onChange).not.toHaveBeenCalled();
    expect(botao()).toHaveTextContent("pressione…");
  });

  it("fora do modo de captura, digitar não muda nada", async () => {
    const { onChange, usuario } = montar();
    await usuario.keyboard("{Meta>}l{/Meta}");
    expect(onChange).not.toHaveBeenCalled();
  });
});

describe("limpar", () => {
  it("o botão de limpar aparece só quando há atalho", () => {
    const { unmount } = render(
      <HotkeyCapture value="" onChange={vi.fn()} t={t} />,
    );
    expect(screen.getAllByRole("button")).toHaveLength(1);
    unmount();

    render(<HotkeyCapture value="SUPER, L" onChange={vi.fn()} t={t} />);
    expect(screen.getAllByRole("button")).toHaveLength(2);
  });

  it("limpar devolve atalho vazio", async () => {
    const onChange = vi.fn();
    render(<HotkeyCapture value="SUPER, L" onChange={onChange} t={t} />);
    await userEvent.setup().click(screen.getAllByRole("button")[1]);
    expect(onChange).toHaveBeenCalledWith("");
  });
});
