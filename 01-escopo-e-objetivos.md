# 01 — Escopo e Objetivos

## Objetivo

Ser o "painel de controle" da customização do Windows: um catálogo de ferramentas da comunidade, cada uma documentada, instalável, ativável e removível com um clique, mais uma biblioteca de temas prontos que combinam várias ferramentas.

## O que o app FAZ

1. **Catálogo de ferramentas**: lista todas as ferramentas suportadas, agrupadas por categoria (window manager, barra de status, taskbar/Start, mods de shell, terminal, launcher, wallpaper, widgets, cursores/ícones).
2. **Instalar / desinstalar** cada ferramenta a partir da fonte oficial (winget, GitHub Releases ou site oficial).
3. **Ativar / desativar** cada ferramenta com um toggle, sem desinstalar (para o processo, remove do autostart, restaura a config anterior quando aplicável).
4. **Documentação embutida**: cada ferramenta tem uma página explicando, em linguagem simples, o que faz, o que muda na tela, atalhos principais, riscos e como desfazer.
5. **Preview** antes de aplicar (screenshots/GIFs e modo "aplicar em teste" com reversão automática).
6. **Biblioteca de temas**: galeria de setups completos (ex.: "Catppuccin Dev", "Minimal macOS-like") que configuram várias ferramentas de uma vez, com aplicação total ou parcial.
7. **Perfis**: salvar o setup atual, alternar entre perfis e exportar como tema compartilhável.
8. **Segurança**: ponto de restauração, backup de configs, rollback e botão "voltar ao Windows original".
9. **Compatibilidade**: detectar a build do Windows e avisar sobre conflitos entre ferramentas.
10. **Widget de acesso rápido**: toggles das ferramentas, troca de perfil e monitor de hardware (CPU, GPU, RAM, disco, rede, temperaturas) sem abrir o app — detalhes no arquivo 11.
11. **Extensível**: adicionar uma ferramenta nova = adicionar um manifesto, sem mexer no núcleo.

## O que o app NÃO faz (fora de escopo)

- Não modifica arquivos de sistema do Windows diretamente (ex.: patch de uxtheme.dll). Só usa ferramentas de terceiros consagradas.
- Não redistribui binários de terceiros.
- Não é um "debloater" nem otimizador de performance (pode virar extensão futura, mas não na v1).
- Não suporta Windows 10 na v1 (pode entrar depois; vários mods são exclusivos do Windows 11).

## Princípios

- **Reversível sempre**: nenhuma ação sem caminho de volta.
- **Explicar antes de fazer**: o usuário vê o que vai mudar antes de confirmar.
- **Iniciante primeiro, avançado disponível**: opções avançadas (editar YAML/CSS direto) existem, mas escondidas atrás de "Advanced".
- **Dados, não código**: ferramentas e temas são descritos em arquivos de dados versionados.
