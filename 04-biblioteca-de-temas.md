# 04 — Biblioteca de Temas

Um **tema** é um setup completo: um conjunto de ferramentas + as configurações de cada uma + wallpaper + paleta de cores. Exemplo: "Catppuccin Dev" = GlazeWM + Zebar + Windhawk (taskbar escondida) + Windows Terminal com Oh My Posh + wallpaper + paleta Catppuccin Mocha.

## Fontes de temas

1. **Biblioteca oficial**: `catalog/themes/` no repositório do projeto, curada via Pull Request.
2. **Importar de URL**: o usuário cola a URL de um repositório GitHub que contenha um `theme.yaml`.
3. **Locais**: temas criados pelo próprio usuário a partir do setup atual ("Salvar como tema").

## Formato do tema (`theme.yaml`)

```yaml
id: catppuccin-dev
name: Catppuccin Dev
author: { name: "William", github: "usuario" }
version: 1.0.0
license: MIT
description:
  en: "Keyboard-driven dev setup with Catppuccin Mocha colors."
  pt-BR: "Setup de dev controlado pelo teclado, com cores Catppuccin Mocha."
tags: [dev, tiling, dark, catppuccin, minimal]
difficulty: intermediate
requires:
  windows_min_build: 22621
palette:                       # usado para sincronizar cores entre ferramentas
  name: catppuccin-mocha
  colors: { base: "#1e1e2e", text: "#cdd6f4", accent: "#cba6f7", red: "#f38ba8" }
components:                    # cada item referencia um manifesto de ferramenta
  - tool: glazewm
    config: configs/glazewm/config.yaml
    required: true
  - tool: zebar
    config: configs/zebar/
    required: true
  - tool: windhawk
    mods:
      - id: windows-11-taskbar-styler
        settings: configs/windhawk/taskbar-styler.json
    required: false
  - tool: oh-my-posh
    config: configs/oh-my-posh/theme.omp.json
    required: false
  - tool: nerd-font
    font: JetBrainsMono
    required: true
wallpaper: assets/wallpaper.png
media:
  cover: screenshots/cover.png
  screenshots: [screenshots/01.png, screenshots/02.png]
  demo: screenshots/demo.mp4
```

## Funcionalidades da biblioteca

- **Galeria** com capa, nome, autor, tags e dificuldade. Filtros por tag, cor predominante, dificuldade, ferramentas usadas e compatibilidade com a build atual.
- **Página do tema**: screenshots/vídeo, lista de ferramentas que serão instaladas (com link para a doc de cada uma), o que muda na tela, tamanho estimado de download.
- **Aplicação total ou parcial**: checkboxes por componente (ex.: aplicar o tema mas sem trocar o window manager). Componentes `required: true` só podem ser desmarcados com aviso.
- **Plano antes de aplicar**: tela de revisão mostrando cada passo (instalar, copiar config, ativar), gerada pelo Transaction Engine.
- **Desfazer tema**: um clique volta ao estado anterior à aplicação.
- **Salvar setup atual como tema**: coleta configs das ferramentas ativas, tira screenshot automático da tela e gera a pasta do tema.
- **Exportar / compartilhar**: gera `.zip` ou orienta a publicar como repositório GitHub, com botão "Enviar para a biblioteca oficial" que abre o fluxo de Pull Request (instruções no CONTRIBUTING.md).
- **Sincronização de paleta (v2)**: aplicar a paleta de um tema sobre o setup atual, recolorindo barra, terminal, bordas do WM e taskbar de uma vez.
- **Favoritos e histórico** de temas aplicados.

## Curadoria e segurança dos temas

- Temas **nunca** contêm executáveis nem scripts arbitrários. Apenas configs (YAML, JSON, CSS, TOML), imagens e fontes de fontes oficiais.
- Qualquer comando executado vem do manifesto da ferramenta, não do tema.
- Temas importados por URL mostram aviso "tema não verificado" e o plano completo antes de aplicar.
- CI da biblioteca oficial valida: schema, ausência de binários, tamanho máximo, screenshots presentes, referências a ferramentas existentes.

## Temas iniciais sugeridos (v1)

1. **Catppuccin Dev** — GlazeWM + Zebar + Windows Terminal/Oh My Posh.
2. **Minimal Clean** — só Windhawk (taskbar translúcida, Start limpo) + Flow Launcher. Para iniciantes.
3. **macOS-like** — barra no topo (YASB) + dock (Seelen UI ou taskbar centralizada flutuante).
4. **Tokyo Night Tiling** — Komorebi + YASB.
5. **Retro Classic** — StartAllBack/ExplorerPatcher estilo Windows 7/10.
