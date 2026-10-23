# 11 — Widget de Acesso Rápido e Informações de Hardware

Além da janela principal, o app oferece um **widget** leve para o dia a dia: ativar/desativar ferramentas e perfis sem abrir o app, e acompanhar o hardware em tempo real.

## Formas de exibição

O usuário escolhe uma ou mais (Configurações → Widget):

1. **Flyout da bandeja**: clique no ícone do app na bandeja do sistema abre um painel compacto (estilo Configurações Rápidas do Windows 11).
2. **Widget flutuante de desktop**: janela pequena, sem borda, que fica no desktop (fixável em canto, opacidade ajustável, "sempre no fundo" ou "sempre no topo").
3. **Atalho global**: abre/fecha o widget (padrão `Win + Alt + W`, configurável).
4. **(v2) Integração com barras**: expor os dados via provider/endpoint local para Zebar e YASB exibirem os mesmos toggles e métricas na barra do topo.

## Seção 1 — Ativações rápidas

- Lista das ferramentas **instaladas** com toggle (mesmo motor do app: Transaction Engine, backup, rollback).
- Usuário escolhe quais aparecem no widget e em que ordem (fixar favoritas).
- Seletor de **perfil** (ex.: Trabalho / Jogos / Minimal) — troca tudo com um clique.
- Botão **"Pausar customizações"**: desativa temporariamente tudo (útil para jogos, apresentações, compartilhar tela) e reativa com outro clique.
- Botão **"Restaurar original"** (com confirmação).
- Indicador de status por ferramenta: ativa / desativada / erro / conflito.
- Toggles que exigem admin ou reinício do Explorer mostram ícone de aviso e pedem confirmação.

## Seção 2 — Informações de hardware

### Métricas

| Grupo | Dados |
|---|---|
| CPU | modelo, uso total e por núcleo, clock atual, temperatura |
| GPU | modelo, uso, VRAM usada/total, temperatura, clock |
| RAM | usada/total, percentual |
| Disco | uso por unidade, leitura/escrita em tempo real, temperatura e saúde (SMART) quando disponível |
| Rede | download/upload em tempo real, IP local, adaptador ativo |
| Sistema | uptime, versão/build do Windows, bateria (notebooks) |
| Ventoinhas | RPM, quando o sensor estiver disponível |

### Exibição
- Modo **compacto** (números e mini barras) e modo **detalhado** (gráficos das últimas 60 s).
- Usuário escolhe quais métricas mostrar e a taxa de atualização (padrão 1 s; mínimo 500 ms).
- Alertas opcionais: temperatura de CPU/GPU acima de um limite, RAM acima de X%.
- Cores do widget seguem a **paleta do tema ativo** (arquivo 04).

### Fonte dos dados
- Uso de CPU/RAM/disco/rede: APIs do Windows (PDH / Performance Counters, WMI) — não exigem admin.
- Temperaturas, clocks, ventoinhas e SMART: **LibreHardwareMonitorLib** (MPL-2.0) rodando num serviço/helper separado, pois a leitura de sensores normalmente exige privilégios de admin.
  - Se o usuário não conceder admin, mostrar só as métricas que não dependem disso e explicar o motivo.
- GPU NVIDIA/AMD/Intel: via LibreHardwareMonitor; fallback para contadores de GPU do Windows (uso) quando sensores não estiverem disponíveis.
- O agente deve verificar a licença e a forma de distribuição da biblioteca escolhida antes de integrar.

### Desempenho
- O widget deve ser leve: meta de **< 1% de CPU** e **< 80 MB de RAM** com atualização a cada 1 s.
- Coleta pausada quando o widget está oculto (exceto se houver alertas ativos).
- Sem telemetria: dados de hardware nunca saem do computador.

## Manifesto e extensibilidade

- Cada ferramenta pode declarar no manifesto (arquivo 03) se aparece no widget por padrão:
  ```yaml
  widget:
    show_by_default: true
    quick_actions:            # ações extras além do toggle (opcional)
      - id: reload-config
        label: { en: "Reload config", pt-BR: "Recarregar configuração" }
        command: "glazewm.exe command wm-reload-config"
  ```
- Métricas de hardware ficam num módulo próprio (`HardwareMonitor`) no core, desacoplado das ferramentas.

## Critérios de aceite

- Ativar/desativar pelo widget tem o mesmo resultado (e o mesmo rollback) que pela janela principal.
- "Pausar customizações" e reativar devolvem exatamente o mesmo estado.
- Métricas sem admin funcionam em qualquer máquina; com admin, temperaturas aparecem em CPUs/GPUs suportadas.
- Consumo dentro das metas de desempenho medido em VM e em máquina real.
