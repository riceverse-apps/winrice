# 09 — Roadmap e Critérios de Aceite

## Fase 0 — Fundação
- Repositório, licença, CI, schemas (`tool.schema.json`, `theme.schema.json`), i18n en/pt-BR.
- App Tauri vazio com navegação lateral, troca de idioma e modo claro/escuro/sistema (arquivo 12).
- **Pronto quando**: CI roda build + validação de schemas; app abre em en e pt-BR, nos modos claro e escuro.

## Fase 1 — Catálogo e toggles (MVP)
- Manifest Loader, Installer (winget), Activator, Transaction Engine, Backup Service.
- 5 ferramentas: GlazeWM, Zebar, Windhawk (com Taskbar Styler), Windows Terminal + Oh My Posh, Flow Launcher.
- Página da ferramenta com Overview, Docs, Shortcuts.
- Botão "Restore original Windows".
- **Pronto quando**: instalar, ativar, desativar e desinstalar cada ferramenta funciona; uma falha forçada no meio de uma operação faz rollback completo; restaurar o Windows original deixa o sistema como antes.

## Fase 2 — Preview, conflitos e compatibilidade
- Try mode com reversão em 30s e atalho de emergência.
- Detecção de build, conflitos, watchdog do Explorer.
- Aba Settings com opções simples do manifesto.
- Restante do catálogo da v1 (arquivo 08).
- **Pronto quando**: ativar duas ferramentas em conflito é bloqueado com explicação; try mode reverte sozinho se não confirmado.

## Fase 3 — Biblioteca de temas
- Theme Loader, galeria, página do tema, aplicação total/parcial, desfazer tema.
- Importar de URL, salvar setup atual como tema, exportar zip.
- 5 temas oficiais iniciais (arquivo 04).
- `index.json` remoto: app recebe temas/ferramentas novos sem atualizar.
- **Pronto quando**: aplicar e desfazer cada tema oficial funciona numa VM limpa de Windows 11.

## Fase 3.5 — Widget e hardware
- Flyout da bandeja e widget flutuante com toggles, perfis e "Pausar customizações".
- Módulo HardwareMonitor (métricas sem admin primeiro; sensores via helper com admin depois).
- **Pronto quando**: critérios do arquivo 11 atendidos.

## Fase 4 — Comunidade e polimento
- Perfis, histórico com desfazer, favoritos.
- Fluxo "enviar tema para a biblioteca oficial" (PR guiado).
- Sincronização de paleta entre ferramentas.
- Publicação no winget, site/landing do projeto.

## Testes
- Testes unitários do Transaction Engine (incluindo falhas simuladas em cada passo).
- Testes de validação de todos os manifestos e temas no CI.
- Roteiro de teste manual em **VM Windows 11 limpa** (Windows Sandbox ou Hyper-V) antes de cada release, cobrindo: instalar, ativar, try mode, aplicar tema, restaurar original.
