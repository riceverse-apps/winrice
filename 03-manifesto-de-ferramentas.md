# 03 — Manifesto de Ferramentas

Cada ferramenta suportada é descrita por um `manifest.yaml` em `catalog/tools/<tool-id>/`. O app não tem lógica específica por ferramenta: tudo vem do manifesto. Se uma ferramenta precisar de algo que o manifesto não cobre, **estender o schema** (em `/schemas/tool.schema.json`) em vez de criar exceção no código.

## Campos

```yaml
id: glazewm                      # único, kebab-case
name: GlazeWM
category: window-manager         # window-manager | status-bar | taskbar-start | shell-mod
                                 # | terminal | launcher | wallpaper | widgets | icons-cursors | file-manager
summary:
  en: "Tiling window manager inspired by i3."
  pt-BR: "Organiza as janelas automaticamente lado a lado, controlado pelo teclado."
difficulty: intermediate         # beginner | intermediate | advanced
homepage: https://github.com/glzr-io/glazewm
license: GPL-3.0                 # SPDX; o app mostra e alerta quando não for livre p/ uso comercial
requires:
  windows_min_build: 22000
  windows_max_build: null        # preencher quando uma build quebrar a ferramenta
  admin: false
install:
  method: winget                 # winget | github-release | url
  id: glzr-io.glazewm
uninstall:
  method: winget
detect:                          # como saber se está instalada/rodando
  process: glazewm.exe
  paths: ["%ProgramFiles%\\glzr.io\\GlazeWM\\glazewm.exe"]
activate:
  start: "glazewm.exe start"
  autostart: true                # cria entrada de inicialização
deactivate:
  stop: "glazewm.exe command wm-exit"
  remove_autostart: true
config:
  paths: ["%USERPROFILE%\\.glzr\\glazewm\\config.yaml"]
  format: yaml
  backup: true
conflicts: [komorebi]            # ids de ferramentas incompatíveis ao mesmo tempo
pairs_well_with: [zebar, yasb, flow-launcher]
changes_on_screen:               # lista didática do que muda visualmente
  - en: "Windows are arranged automatically without overlapping."
    pt-BR: "As janelas são organizadas automaticamente, sem ficar uma em cima da outra."
shortcuts:
  - keys: "Alt + H/J/K/L"
    en: "Move focus between windows"
    pt-BR: "Muda o foco entre as janelas"
risks:
  en: "Some apps (games, fullscreen) may need a rule to float."
  pt-BR: "Alguns apps (jogos, tela cheia) podem precisar de uma regra para não serem organizados."
how_to_undo:
  pt-BR: "Desative no toggle ou pressione Alt+Shift+E."
media:
  screenshots: [screenshots/01.png, screenshots/02.png]
  demo: screenshots/demo.gif
settings:                        # opções simples expostas na UI (sem editar YAML)
  - key: gaps.inner_gap
    type: number
    label: { en: "Gap between windows", pt-BR: "Espaço entre janelas" }
    default: 10
```

## Documentação longa

Além do manifesto, cada ferramenta tem `docs/en.md` e `docs/pt-BR.md` com a explicação completa (o que é, para quem serve, primeiros passos, dicas, FAQ). O app renderiza esses arquivos na página da ferramenta.

## Validação

- JSON Schema em `/schemas/tool.schema.json`.
- CI valida todo manifesto em PR: schema, links ativos, existência de screenshots, tradução pt-BR presente, `id` único, `conflicts` apontando para ids existentes.
