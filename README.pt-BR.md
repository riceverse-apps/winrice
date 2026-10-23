# WinRice

Personalize o Windows 11 com um clique: um catálogo de ferramentas de "rice" da comunidade (gerenciadores de janelas em mosaico, barras de status, mods do shell, prompts de terminal, lançadores), uma biblioteca de temas e um widget de acesso rápido. Toda mudança mostra o plano antes, recebe backup e pode ser desfeita.

[English](README.md)

> **Status: em desenvolvimento, ainda sem versão pública.** O código compila e os testes automáticos passam, mas os fluxos de instalar / ligar / desfazer **ainda não foram validados** numa VM limpa de Windows 11. Não rode numa máquina em que você não possa experimentar. O que ainda precisa ser confirmado está marcado como `UNVERIFIED` nos manifestos.

## O que faz

- **Catálogo de ferramentas** (20): instalar, ligar/desligar e desinstalar com um botão. Cada ferramenta tem documentação em linguagem simples, em inglês e português.
- **Seguro por construção**: toda mudança passa por um único Transaction Engine (plano → backup → aplicar → verificar → confirmar ou desfazer). Se um passo falhar, tudo o que já foi feito é desfeito.
- **Modo de teste**: aplica por 30 segundos; se você não confirmar, volta sozinho. Atalho de emergência: `Ctrl + Alt + Shift + R`.
- **Conflitos e compatibilidade**: ferramentas que conflitam são bloqueadas com explicação (ou o outro lado é desligado para você); ferramentas que não suportam sua build do Windows ficam desativadas. Um watchdog do Explorer desfaz loop de travamentos.
- **Biblioteca de temas**: 5 temas oficiais, aplicação total ou parcial, desfazer, importar por link do GitHub, salvar seu setup como tema, exportar `.zip`, sincronizar a paleta e um fluxo guiado para enviar um tema à biblioteca oficial.
- **Perfis e "Pausar customizações"**: troque o conjunto de ferramentas ligadas com um clique.
- **Widget**: painel na bandeja e widget flutuante com ligar/desligar rápido, perfis e informações de hardware em tempo real (CPU, RAM, discos, rede, bateria). Temperaturas, ventoinhas e uso da GPU exigem um helper com privilégios que **ainda não existe**.
- **Histórico com desfazer** (da mudança mais recente para trás), favoritos, modo claro/escuro/sistema e cor de destaque.
- **Sem telemetria.** Nada sai do seu computador, exceto os downloads que você pede (GitHub, winget).

## Desenvolvimento

Requisitos: Node 20+, Rust (stable, MSVC) e as Build Tools do Visual Studio com C++, WebView2 (já vem no Windows 11).

```bash
npm install                          # ferramentas do catálogo
npm run validate                     # idiomas, manifestos, temas, glossário; gera catalog/index.json
npm install --prefix app
npm --prefix app run dev             # só a interface web, http://localhost:1420
npm --prefix app run tauri -- dev    # app de desktop
cargo test --manifest-path app/src-tauri/Cargo.toml
```

Os testes nunca mexem na sua máquina: o Transaction Engine roda contra uma simulação do sistema operacional em memória. Testar de verdade é feito numa VM ou no Windows Sandbox (veja o [CONTRIBUTING](CONTRIBUTING.md)).

## Estrutura do repositório

| Pasta | Conteúdo |
|---|---|
| `app/` | app Tauri 2 (Rust) + Vue 3 |
| `catalog/tools/` | uma pasta por ferramenta: `manifest.yaml`, `docs/en.md`, `docs/pt-BR.md` |
| `catalog/themes/` | temas oficiais |
| `catalog/glossary/` | jargões explicados em linguagem simples |
| `schemas/` | JSON Schemas dos manifestos e temas |
| `locales/` | `en.json`, `pt-BR.json` |
| `branding/` | logos e ícones |
| `docs/` | documentação do projeto (em inglês) |
| `packaging/`, `site/` | manifestos do winget, landing page |

## Licença

MIT, veja [LICENSE](LICENSE). As ferramentas de terceiros mantêm suas próprias licenças; o WinRice nunca embute os binários delas, apenas as instala a partir das fontes oficiais.
