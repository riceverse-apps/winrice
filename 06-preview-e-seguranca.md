# 06 — Preview, Segurança e Compatibilidade

## Preview

Não é possível simular com fidelidade como o desktop vai ficar sem aplicar de verdade. Por isso, dois níveis:

1. **Preview estático (sempre disponível)**: screenshots, GIFs e vídeos do manifesto/tema, mais a lista "o que muda na tela". Opcional (v2): sobrepor os screenshots ao wallpaper atual do usuário para dar noção do resultado.
2. **Try mode / aplicar em teste**: aplica de verdade via Transaction Engine e mostra uma janela flutuante "Manter alterações? Revertendo em 30s" (igual à troca de resolução do Windows). Se o usuário não confirmar, rollback automático.
   - A janela de confirmação precisa ficar sempre visível (topmost) e funcionar mesmo se a taskbar sumir.
   - Atalho global de emergência para reverter: `Ctrl + Alt + Shift + R`.

## Backup e rollback

- **Ponto de restauração do Windows** antes da primeira operação do dia que mexa no shell (configurável). Requer admin; se negado, seguir com backup próprio e avisar.
- **Backup de configs**: antes de sobrescrever qualquer arquivo de config, copiar para `%APPDATA%\WinRice\backups\<timestamp>\`.
- **Backup de registro**: exportar as chaves que serão alteradas (ex.: menu de contexto, autostart).
- **Rollback transacional** conforme 02 (Transaction Engine).
- **Restore original Windows**: desativa todas as ferramentas, desinstala as que o app instalou (perguntando), restaura todos os backups originais e reinicia o Explorer.
- **Watchdog**: se o Explorer travar em loop após uma operação, desativar automaticamente a última ferramenta aplicada.

## Conflitos

- Definidos no manifesto (`conflicts`). Ao ativar uma ferramenta em conflito com outra ativa, o app oferece: desativar a outra ou cancelar.
- Conflitos conhecidos iniciais: GlazeWM × Komorebi; ExplorerPatcher × StartAllBack; Start11 × StartAllBack; múltiplos mods que escondem/movem a taskbar.

## Compatibilidade com o Windows

- Detectar build e edição do Windows na inicialização.
- Ferramentas fora da faixa `windows_min_build`/`windows_max_build` aparecem como "Incompatível com sua versão" e não podem ser ativadas (opção avançada para forçar, com aviso).
- Após update do Windows (build mudou), mostrar aviso na Home e checar o catálogo atualizado por novas restrições.

## Permissões e segurança

- O app roda sem admin; pede elevação só para passos que exigem (via helper separado), explicando o motivo.
- Downloads apenas das fontes declaradas no manifesto; verificar hash quando a fonte publicar.
- Nunca executar comandos vindos de temas importados; somente comandos dos manifestos oficiais.
- Telemetria: nenhuma por padrão.
