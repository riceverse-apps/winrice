# 14 — Identidade Visual e Logos

Família **Riceverse** (organização GitHub `riceverse-apps`): **WinRice**, **MacRice** e **LinuxRice**. Os três logos devem parecer da mesma família, mudando só a cor de destaque e um detalhe de forma.

## Conceito

- "Rice" vem de *ricing*, termo da comunidade para customizar o visual do sistema.
- Símbolo base: **um grão de arroz estilizado** (forma oval alongada, levemente inclinada, cantos suaves), geométrico e minimalista.
- Cada app ganha um detalhe dentro ou ao redor do grão que remete à sua plataforma **sem usar o logo oficial do sistema**:
  - **WinRice**: o grão dividido em blocos/painéis, lembrando janelas organizadas lado a lado (tiling).
  - **MacRice**: o grão com um brilho suave e um ponto de destaque, visual polido e "vidro".
  - **LinuxRice**: o grão com linhas de terminal (um `>_` discreto ou listras de código).
- Estilo: flat ou com gradiente sutil, sem sombras pesadas, sem texto dentro do ícone, legível a 16×16 px.

## Proibido (questão legal)

- Não usar o logo do Windows (quatro quadrados com as mesmas proporções e cores), a maçã da Apple ou o pinguim Tux.
- Não usar as cores oficiais exatas de marca da Microsoft ou da Apple como cor principal.
- Não usar "Windows", "Mac" ou "Linux" escritos com a tipografia oficial dessas marcas.

## Paleta

| Uso | Cor | Hex sugerido |
|---|---|---|
| Grão (cor base, comum aos três) | Creme / arroz | `#F5EFE0` |
| Contorno/escuro (comum) | Grafite | `#1E1E2E` |
| Destaque WinRice | Azul-violeta | `#7C6FF0` |
| Destaque MacRice | Rosa-coral | `#F28FAD` |
| Destaque LinuxRice | Âmbar | `#F5A742` |

O app usa a cor de destaque do próprio logo como accent padrão quando o usuário escolhe "Custom" (arquivo 12).

## Tipografia do nome (wordmark)

- Fonte sans-serif geométrica, gratuita e com licença aberta: **Inter**, **Outfit** ou **Space Grotesk**.
- Escrita: `WinRice` (W e R maiúsculos), peso semibold. "Win" em grafite e "Rice" na cor de destaque, ou o inverso.

## Arquivos necessários

Criar uma pasta `/branding` no repositório com:

| Arquivo | Tamanho / formato | Uso |
|---|---|---|
| `logo-mark.svg` | vetor, master | Fonte de todos os outros |
| `logo-mark-1024.png` | 1024×1024, fundo transparente | Base para gerar ícones |
| `logo-full.svg` | vetor (símbolo + nome) | README, site |
| `logo-full-dark.svg` / `logo-full-light.svg` | vetor | Versões para fundo escuro e claro |
| `icon.ico` | 16, 24, 32, 48, 64, 128, 256 px | Executável e instalador do Windows |
| Ícones do Tauri | `32x32.png`, `128x128.png`, `128x128@2x.png`, `icon.ico`, `Square*Logo.png` | Gerados com `tauri icon logo-mark-1024.png` |
| `tray-light.svg` / `tray-dark.svg` | monocromático, 16 e 32 px | Ícone da bandeja (versão para tema claro e escuro) |
| `avatar-github.png` | 500×500 | Avatar da organização e dos repositórios |
| `social-preview.png` | 1280×640 | Preview do repositório ao compartilhar link |
| `readme-banner.png` | 1600×400 | Topo do README (versões light e dark) |

## Regras para o agente

1. Se a pasta `/branding` já existir com os arquivos, usar esses arquivos. **Não redesenhar o logo.**
2. Se receber só uma imagem PNG/JPG gerada por IA, vetorizar para SVG (ou pedir ao dono que vetorize), limpar e gerar os demais tamanhos a partir dela.
3. Se não houver logo nenhum ainda, criar um **placeholder simples em SVG** seguindo este arquivo (grão de arroz + cor de destaque) e marcar como provisório no README, para não travar o desenvolvimento.
4. Conferir o ícone em 16×16 e 32×32 nos temas claro e escuro do Windows antes de considerar pronto.
