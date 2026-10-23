# WinRice — Especificação do Projeto

Aplicativo desktop para Windows 11 que reúne, num só lugar, as ferramentas de customização visual da comunidade (tiling window managers, barras de status, mods de shell, terminal, wallpapers etc.), permitindo **ativar e desativar cada uma com um clique**, **ver antes de aplicar**, **entender o que cada coisa faz**, **aplicar temas completos de uma biblioteca** e usar um **widget de acesso rápido** com toggles e informações de hardware.

Público-alvo: pessoas que querem customizar o Windows mas não têm conhecimento técnico suficiente para montar um setup sozinhas, e devs que querem um setup estilo Linux sem perder horas configurando.

## Índice

| Arquivo | Assunto |
|---|---|
| 01-escopo-e-objetivos.md | O que o app faz e o que não faz |
| 02-arquitetura.md | Stack, componentes e estrutura do repositório |
| 03-manifesto-de-ferramentas.md | Formato do arquivo que descreve cada ferramenta |
| 04-biblioteca-de-temas.md | Biblioteca de temas: formato, galeria, aplicação, compartilhamento |
| 05-telas-e-ux.md | Telas do aplicativo |
| 06-preview-e-seguranca.md | Preview, backup, rollback, conflitos e compatibilidade |
| 07-idiomas-e-documentacao.md | Inglês + pt-BR e documentação embutida |
| 08-catalogo-inicial.md | Ferramentas que entram na primeira versão |
| 09-roadmap-e-criterios-de-aceite.md | Fases de entrega e critérios de pronto |
| 10-guia-para-o-agente.md | Regras de trabalho e como adicionar ferramentas/temas |
| 11-widget-e-hardware.md | Widget de acesso rápido (toggles, perfis) e monitor de hardware |
| 12-aparencia-do-app.md | Modo claro/escuro, cor de destaque e aparência do próprio app |
| 13-familia-rice.md | Família Riceverse: MacRice e LinuxRice como projetos separados |
| 14-identidade-visual.md | Conceito do logo, cores, tipografia e arquivos de ícone necessários |
| 15-prompts-de-logo.md | Prompts para gerar os logos numa IA de imagem |

## Regras gerais para o agente

- Código, nomes de arquivos, commits, README do repositório e textos padrão da interface: **em inglês**.
- Todo texto visível ao usuário passa por i18n, com tradução **pt-BR completa** desde a primeira versão.
- Projeto **open source no GitHub**, repositório `riceverse-apps/winrice` (privado até a primeira versão apresentável). Nunca incluir executáveis de terceiros no repositório.
- Leia todos os arquivos desta pasta antes de começar. Em caso de conflito entre arquivos, o mais específico vence (ex.: 06 vence 02 em assuntos de segurança).

## Referências de mercado

- **OttoRice**: instala setups completos a partir de manifesto JSON com backup transacional e rollback. Inspiração para a biblioteca de temas e para o motor de aplicação. Nosso diferencial: controle granular por ferramenta, documentação didática embutida e foco em iniciantes.
- **Translucent-Win11** e repositórios de dotfiles ("windots"): guias manuais da comunidade. Fonte de temas para importar.
