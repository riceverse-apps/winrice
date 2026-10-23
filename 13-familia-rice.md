# 13 — Família Riceverse (WinRice, MacRice, LinuxRice)

Decisão do dono do projeto: **cada sistema operacional terá seu próprio projeto, independente**. Windows, macOS e Linux são mundos diferentes (ferramentas, permissões, formas de customizar), então não haverá núcleo de código compartilhado nem monorepo.

## O que isso significa para a v1 (Windows)

- O projeto é **só para Windows**. Não criar abstrações "multiplataforma", traits genéricos ou pastas reservadas para macOS/Linux.
- Pode usar APIs e recursos específicos do Windows livremente (registro, winget, Mica, WinUI se fizer sentido).
- Estrutura de repositório: a definida no arquivo 02, sem divisão por plataforma.
- Nomes definidos: **WinRice** (este projeto), **MacRice** e **LinuxRice**, cada um com seu repositório na organização GitHub **`riceverse-apps`**:
  - `riceverse-apps/winrice`
  - `riceverse-apps/macrice`
  - `riceverse-apps/linuxrice`
- Os repositórios começam privados e serão tornados públicos quando houver uma primeira versão apresentável (README, LICENSE e app abrindo).

## O que vale manter parecido entre os projetos (opcional, sem compartilhar código)

Para que no futuro a experiência seja familiar entre os apps, só por convenção:

- Mesmos princípios: reversível sempre, explicar antes de fazer, iniciante primeiro (arquivo 01).
- Formato de tema e manifesto com estrutura parecida (campos `id`, `name`, `summary`, `palette`, `components`), para facilitar portar temas e documentação.
- Mesma identidade visual (logo, cores, tipografia) e mesmo padrão de idiomas (inglês + pt-BR).
- Mesma estrutura de documentação embutida por ferramenta (arquivo 07).

Nada disso deve atrasar ou complicar a v1 do Windows.
