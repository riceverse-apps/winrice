# 15 — Prompts para Gerar os Logos em IA de Imagem

Prompts prontos para usar em ChatGPT (gpt-image), Gemini (Nano Banana), Midjourney, Ideogram ou similar. Os prompts estão em **inglês** porque as IAs de imagem respondem melhor assim; a explicação de cada um está em português.

## Dicas antes de começar

- Gere **várias variações** de cada prompt e escolha a melhor; depois peça ajustes ("make it simpler", "thicker shapes", "remove the gradient").
- Gere primeiro o **WinRice**. Quando gostar, use a imagem dele como referência para gerar MacRice e LinuxRice ("same style as the reference image, but...") para manter a família.
- IAs de imagem erram texto com frequência. Gere o **símbolo sem texto** e coloque o nome depois, com a fonte (Inter/Outfit/Space Grotesk) num editor.
- O resultado sai em PNG. Para virar SVG: vetorizar no Figma/Illustrator ou numa ferramenta de vetorização automática, e limpar os detalhes.

---

## 1. WinRice — símbolo (ícone do app)

*O grão de arroz dividido em blocos, lembrando janelas organizadas lado a lado.*

```
Minimalist app icon, flat vector style. A single stylized rice grain, elongated oval shape, slightly tilted, with soft rounded edges. The grain is divided into 3 or 4 neat rectangular tiles separated by thin gaps, suggesting tiled windows on a desktop. Rice grain color warm cream #F5EFE0, tiles accented in blue-violet #7C6FF0, thin dark graphite outline #1E1E2E. Centered on a transparent background, generous padding, no text, no shadows, no 3D, clean geometric shapes, readable at 16x16 pixels. Do not use the Microsoft Windows logo or its four-square layout.
```

## 2. MacRice — símbolo

*O mesmo grão, com brilho suave e acabamento polido.*

```
Minimalist app icon, flat vector style, same family as the reference icon. A single stylized rice grain, elongated oval shape, slightly tilted, soft rounded edges. Smooth glossy look with one subtle highlight streak and a small glowing dot accent, suggesting a polished glass interface. Rice grain color warm cream #F5EFE0, accent in coral pink #F28FAD, thin dark graphite outline #1E1E2E. Transparent background, generous padding, no text, no heavy shadows, readable at 16x16 pixels. Do not use an apple shape or any Apple logo.
```

## 3. LinuxRice — símbolo

*O mesmo grão, com um detalhe de terminal.*

```
Minimalist app icon, flat vector style, same family as the reference icon. A single stylized rice grain, elongated oval shape, slightly tilted, soft rounded edges. Inside the grain, a small terminal prompt symbol ">_" and two thin horizontal code lines. Rice grain color warm cream #F5EFE0, accent in amber #F5A742, thin dark graphite outline #1E1E2E. Transparent background, generous padding, no text besides the ">_" symbol, no shadows, readable at 16x16 pixels. Do not use a penguin or the Tux mascot.
```

## 4. Riceverse — logo da organização (avatar do GitHub)

*Os três grãos juntos, representando a família.*

```
Minimalist logo, flat vector style. Three stylized rice grains arranged in a balanced fan or orbit composition, each grain warm cream #F5EFE0 with a thin graphite outline #1E1E2E and a small accent of a different color: blue-violet #7C6FF0, coral pink #F28FAD and amber #F5A742. Dark graphite background #1E1E2E, centered, square format, no text, clean geometric shapes, works as a small social media avatar.
```

## 5. Ícone da bandeja (tray), monocromático

*Versão simplificada em uma cor só, para a área de notificação do Windows.*

```
Monochrome system tray icon, single solid color white on transparent background. A stylized rice grain silhouette, elongated oval, slightly tilted, divided into 3 tiles by thin gaps. Ultra simple, bold shapes, no outline details, must remain clear at 16x16 pixels.
```

Depois, gerar a versão preta (para tema claro) só invertendo a cor no editor.

## 6. Banner do README

```
Wide banner 1600x400, dark graphite background #1E1E2E with a subtle blue-violet #7C6FF0 gradient glow on the right. On the left, the WinRice icon (stylized rice grain divided into tiles). On the right side, a blurred stylized mockup of a customized desktop with tiled windows and a top status bar. Leave clear empty space in the center for the title text to be added later. Flat modern style, no real brand logos.
```

## Checklist final antes de entregar ao agente

- [ ] Os três símbolos parecem da mesma família
- [ ] Nenhum lembra o logo oficial do Windows, a maçã ou o Tux
- [ ] Legíveis em tamanho pequeno (reduza para 32 px e veja)
- [ ] Fundo transparente nos ícones
- [ ] Salvos em `/branding` com os nomes do arquivo 14
