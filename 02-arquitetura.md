# 02 — Arquitetura

## Stack recomendada

- **Tauri 2** (núcleo em Rust) + **frontend Vue 3 ou React com TypeScript**.
  - Motivo: app leve, instalador pequeno, interface web fácil de manter e revisar.
  - Alternativa aceitável: WinUI 3 + .NET 8, se o agente justificar (visual nativo mais fiel ao Windows 11).
- Estilo visual: seguir Fluent Design (Mica/Acrylic onde o Tauri permitir), com modo claro, escuro e seguir o sistema (detalhes no arquivo 12).
- Projeto exclusivo para Windows; versões de macOS e Linux serão projetos separados (arquivo 13).
- Armazenamento local: SQLite (estado, histórico de operações, perfis) + arquivos em `%APPDATA%\WinRice\`.

## Componentes

```
┌──────────────────────── UI (Vue/React) ────────────────────────┐
│ Catálogo │ Página da ferramenta │ Biblioteca de temas │ Perfis │
└───────────────────────────────┬────────────────────────────────┘
                                │ comandos (Tauri IPC)
┌───────────────────────────────┴────────────────────────────────┐
│ Core (Rust)                                                    │
│  ├─ Manifest Loader      lê/valida manifestos de ferramentas    │
│  ├─ Theme Loader         lê/valida temas (local e remoto)       │
│  ├─ Installer            winget / GitHub Releases / URL oficial │
│  ├─ Activator            start/stop, autostart, configs         │
│  ├─ Transaction Engine   plano → backup → aplica → rollback     │
│  ├─ Compatibility        build do Windows, conflitos            │
│  ├─ Backup Service       configs, registro, ponto de restauração│
│  └─ Elevation Helper     pede admin só quando necessário        │
└────────────────────────────────────────────────────────────────┘
```

## Transaction Engine (coração do app)

Toda mudança (ativar ferramenta, aplicar tema, trocar perfil) segue o mesmo fluxo:

1. **Plan**: gera a lista de passos (instalar X, copiar config Y, ativar mod Z) e mostra ao usuário.
2. **Backup**: salva tudo que será alterado.
3. **Apply**: executa passo a passo, registrando cada um.
4. **Verify**: confere se o processo subiu / config foi aplicada.
5. **Commit ou Rollback**: se algo falhar, desfaz os passos já executados em ordem reversa.

Cada operação vira um registro no histórico, com opção de desfazer depois.

## Estrutura do repositório

```
/app                 código do aplicativo (Tauri)
/catalog
  /tools/<tool-id>/  manifest.yaml, docs/en.md, docs/pt-BR.md, screenshots/
  /themes/<theme-id>/ theme.yaml, configs/, screenshots/, README.md
  index.json         índice gerado automaticamente (CI)
/schemas             JSON Schema dos manifestos e temas
/docs                documentação do projeto (en)
/locales             en.json, pt-BR.json
/.github/workflows   CI: build, testes, validação de manifestos, geração do index
```

O catálogo fica no mesmo repositório na v1. O app baixa `catalog/index.json` do GitHub (branch `main` ou release) para receber ferramentas e temas novos **sem precisar atualizar o app**.

## Distribuição

- Instalador MSI/NSIS gerado pelo Tauri, publicado em GitHub Releases.
- Publicar também no winget.
- Assinatura de código é desejável (evita alerta do SmartScreen); documentar no README enquanto não houver.
