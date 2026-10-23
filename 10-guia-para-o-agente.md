# 10 — Guia para o Agente

## Regras de trabalho

1. Leia todos os arquivos desta especificação antes de começar.
2. Trabalhe por fases (arquivo 09). Ao fim de cada fase, pare, resuma o que foi feito e o que falta, e aguarde aprovação.
3. Código, commits e docs do repositório em inglês; mensagens de resumo para o dono do projeto em **português**.
4. Nunca baixar, embutir ou commitar binários de terceiros.
5. Nunca alterar arquivos de sistema do Windows diretamente.
6. Toda operação que muda o sistema passa pelo Transaction Engine. Sem exceções.
7. Testar mudanças de shell somente em VM/Windows Sandbox, nunca na máquina principal do dono.
8. Antes de criar o manifesto de uma ferramenta, verificar na fonte oficial: id do winget, licença, versões do Windows suportadas, caminhos de config.
9. Na dúvida sobre comportamento de uma ferramenta, documentar a dúvida no PR em vez de supor.

## Como adicionar uma ferramenta nova

Pedido típico do dono: *"Adicione a ferramenta X (link)."*

1. Pesquisar a ferramenta na fonte oficial.
2. Criar `catalog/tools/<id>/manifest.yaml` seguindo o arquivo 03.
3. Criar `docs/en.md` e `docs/pt-BR.md` com a estrutura do arquivo 07.
4. Adicionar screenshots/GIF (do repositório oficial, respeitando a licença, ou capturados em VM).
5. Declarar `conflicts` e `pairs_well_with`, revisando também os manifestos existentes (conflito é bidirecional).
6. Se o manifesto não comportar algo da ferramenta, propor extensão do schema.
7. Rodar a validação do CI e testar install/ativar/desativar/desinstalar em VM.
8. Abrir PR com resumo em português do que a ferramenta faz e dos riscos.

## Como adicionar um tema novo

Pedido típico: *"Crie um tema X com as ferramentas A, B, C"* ou *"Importe o tema deste repositório (link)."*

1. Confirmar que todas as ferramentas do tema já têm manifesto (se não, adicionar primeiro).
2. Criar `catalog/themes/<id>/theme.yaml` seguindo o arquivo 04.
3. Incluir apenas configs, imagens e referências a fontes — nada executável.
4. Ao importar tema de terceiros: verificar a licença do repositório original, dar crédito ao autor no `theme.yaml` e no README do tema.
5. Gerar capa e screenshots aplicando o tema em VM.
6. Testar aplicar e desfazer em VM limpa.

## Template de pedido (para o dono usar)

```
Adicione ao catálogo: <nome da ferramenta>
Link oficial: <url>
Categoria (se souber): <categoria>
Observações: <o que eu quero que ela faça / por que quero>
```
