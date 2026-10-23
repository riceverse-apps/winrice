# 07 — Idiomas e Documentação

## Idiomas

- Idioma base: **inglês** (código, repositório, textos padrão).
- **pt-BR obrigatório** desde a v1, com 100% de cobertura.
- Todo texto de interface em `/locales/<lang>.json`; nenhum texto fixo no código.
- Manifestos e temas trazem campos traduzidos (`en`, `pt-BR`); se faltar tradução, cair para `en` e mostrar selo "não traduzido".
- O app detecta o idioma do Windows na primeira execução; pode ser trocado nas configurações.
- Estrutura preparada para outros idiomas via contribuição (ex.: es, fr).

## Documentação embutida (dentro do app)

Cada ferramenta tem `docs/en.md` e `docs/pt-BR.md` com esta estrutura padrão:

1. O que é (2–3 frases, sem jargão)
2. Para quem serve
3. O que muda na sua tela
4. Primeiros passos depois de ativar
5. Atalhos principais
6. Personalizações comuns
7. Problemas comuns e soluções
8. Como desfazer
9. Links oficiais

Há também um **glossário** (`/catalog/glossary/`) com termos como tiling, status bar, rice, shell, Nerd Font, dotfiles; termos do glossário viram tooltips automáticos na interface.

## Documentação do repositório (GitHub, em inglês)

Seguindo a convenção de um arquivo por assunto:

- `README.md` — visão geral, screenshots, instalação
- `docs/architecture.md`
- `docs/adding-a-tool.md`
- `docs/creating-a-theme.md`
- `docs/manifest-reference.md`
- `docs/theme-reference.md`
- `docs/translating.md`
- `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`, `SECURITY.md`, `LICENSE` (sugestão: MIT)
- `README.pt-BR.md` — versão em português do README
