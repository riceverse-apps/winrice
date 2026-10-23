# 12 — Aparência do Próprio App

O WinRice é um app de customização, então ele mesmo precisa ser bonito e personalizável. Este arquivo trata da aparência **do app** (janela principal e widget), não dos temas que ele aplica no Windows (arquivo 04).

## Modo claro e escuro

Opção em Configurações → Aparência, com três valores:

| Opção | Comportamento |
|---|---|
| **System** (padrão) | Segue o modo do Windows e troca na hora quando o usuário muda o sistema, sem reiniciar o app |
| **Light** | Sempre claro |
| **Dark** | Sempre escuro |

- Atalho para alternar rápido: `Ctrl + Shift + L` dentro do app.
- A escolha vale para a janela principal, o widget, o flyout da bandeja e as janelas de confirmação (try mode).
- Sem "flash" de tela branca ao abrir no modo escuro: o tema é aplicado antes da janela aparecer.

## Cor de destaque (accent)

- **System**: usa a cor de destaque do Windows (padrão).
- **Custom**: o usuário escolhe uma cor.
- **From active theme**: usa a cor `accent` da paleta do tema aplicado pela biblioteca (arquivo 04). Assim o app combina com o setup do usuário.

## Material de fundo

- Mica (padrão), Acrylic ou Sólido.
- "Sólido" é a opção para quem prefere desempenho ou acessibilidade (sem transparência).
- Respeitar a configuração "Efeitos de transparência" do Windows: se estiver desligada, usar fundo sólido.

## Outras opções de aparência

- **Densidade**: Confortável (padrão) / Compacta.
- **Tamanho da fonte da interface**: respeitar a escala de texto do Windows, com ajuste extra no app.
- **Reduzir animações**: seguir a configuração de animações do Windows, com opção própria.
- **Alto contraste**: quando o Windows estiver em modo de alto contraste, o app usa as cores do sistema e desativa transparências.

## Regras de implementação

- Todas as cores vêm de **design tokens** (variáveis CSS), definidos uma vez para claro e uma vez para escuro. Nenhuma cor fixa espalhada no código.
- Tokens mínimos: `bg`, `bg-elevated`, `surface`, `text`, `text-muted`, `border`, `accent`, `accent-text`, `success`, `warning`, `danger`, `focus-ring`.
- Contraste mínimo **WCAG AA** nos dois modos, inclusive com cor de destaque personalizada (se a cor escolhida não tiver contraste suficiente, ajustar automaticamente o tom do texto sobre ela).
- Screenshots e GIFs da documentação do repositório devem existir nas versões clara e escura.
- Ícones em SVG que herdam a cor do texto, para funcionar nos dois modos.

## Critérios de aceite

- Trocar entre Light, Dark e System atualiza todas as janelas abertas instantaneamente.
- Com "System", mudar o modo do Windows reflete no app sem reiniciar.
- Nenhuma tela com texto ilegível ou contraste abaixo de AA em nenhum dos modos.
- A preferência é salva e restaurada ao reabrir o app.
