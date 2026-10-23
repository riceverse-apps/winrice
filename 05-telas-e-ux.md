# 05 — Telas e UX

Navegação lateral (estilo Configurações do Windows 11) com as seções abaixo.

## 1. Início (Home)
- Status do setup atual: quais ferramentas estão ativas, perfil ativo, tema aplicado.
- Atalhos: "Explorar temas", "Ver ferramentas", "Voltar ao Windows original".
- Avisos: update do Windows detectado que afeta alguma ferramenta, atualizações disponíveis.

## 2. Ferramentas (Catálogo)
- Cards agrupados por categoria, com ícone, nome, resumo de uma linha, selo de dificuldade e **toggle de ativar/desativar**.
- Estado do card: não instalada / instalada e desativada / ativa / incompatível / em conflito.
- Busca e filtros (categoria, dificuldade, instaladas).

## 3. Página da ferramenta
- Cabeçalho: nome, toggle, botão instalar/desinstalar, licença, link oficial.
- Abas:
  - **Overview**: o que é, o que muda na tela (lista didática), screenshots/GIF.
  - **Settings**: opções simples vindas do manifesto (sliders, cores, switches), com "Advanced" para abrir o arquivo de config.
  - **Shortcuts**: tabela de atalhos.
  - **Docs**: documentação completa renderizada (pt-BR ou en).
  - **Compatibility**: builds suportadas, conflitos, funciona bem com.
- Botão **"Preview"** (ver 06).

## 4. Temas (Biblioteca)
- Galeria em grid com capas grandes, filtros e busca.
- Abas: Oficiais / Importados / Meus temas / Favoritos.
- Botão "Importar de URL".
- Página do tema conforme 04.

## 5. Perfis
- Lista de perfis salvos (ex.: "Trabalho", "Jogos" — jogos com tudo desativado).
- Trocar perfil com um clique; exportar perfil como tema.

## 6. Histórico
- Linha do tempo de operações (aplicou tema X, ativou Y), cada uma com "Desfazer".

## 7. Configurações do app
- Idioma (English / Português (Brasil)), aparência do app (System / Light / Dark, cor de destaque, material — ver arquivo 12), criar ponto de restauração automaticamente (on por padrão), canal do catálogo (estável/beta), pasta de backups.
- **"Restore original Windows"**: desativa e remove tudo que o app instalou, restaura backups.

## Diretrizes de UX
- Toda ação destrutiva ou que reinicia o Explorer mostra confirmação com o plano.
- Linguagem simples nos textos pt-BR; evitar jargão ("tiling" sempre acompanhado de explicação na primeira menção).
- Tooltips com "?" ao lado de termos técnicos.
- Primeira execução: onboarding curto (3 telas) explicando toggle, preview e "voltar ao original", e oferecendo temas para iniciantes.
- Acessibilidade: navegação por teclado completa, contraste WCAG AA, compatível com leitor de tela.
