# Vídeos do QiBuilder: como extrair o máximo deles

O Jimmy grava o trabalho dele no QiBuilder (o programa que este fork vai substituir) narrando o que faz. Cada vídeo passa por
três fontes que se completam:

| Fonte | O que dá | Como |
|---|---|---|
| **Imagem** | *Como* ele trabalha: comandos, abas, janelas, a ordem dos passos | quadros-chave a cada mudança de tela |
| **Fala** | *Por quê* e *o que dói*: intenção, tempo perdido, o que quer automatizar | transcrição (Whisper) com marcação de tempo |
| **Modelo** (conector MCP da AltoQi) | *O que* o projeto tem por dentro: elementos, peças, propriedades, catálogo de peças | consultas de leitura ao projeto aberto no QiBuilder |

A imagem sozinha mostra cliques sem intenção; a fala sozinha cita "esse aqui" sem dizer o quê; o modelo dá os dados exatos
(o nome da peça, o diâmetro, a altura) que a tela mostra pequeno demais para ler.

## O processo, a cada vídeo novo

1. **Buscar.** Listar a pasta de vídeos do Drive dele (pelo computador ligado à sessão) e trazer os vídeos que ainda não têm
   `.txt` ao lado. Até ~100 MB por arquivo a cópia funciona; acima disso, pedir para exportar em 720p ou dividir.
2. **Processar.** Rodar a ferramenta:

   ```sh
   python3 tools/videos/analisar_video.py "VIDEO.mp4" --saida ANALISES \
       --vocabulario tools/videos/vocabulario.txt --vocabulario VOCABULARIO-PRIVADO.txt
   ```

   Ela escreve, por vídeo: a transcrição (`.txt` e legenda `.srt`), os quadros-chave (um depois de cada mudança de tela, já
   assentada, e pelo menos um a cada 15 s), folhas de quadros com o tempo marcado, o `roteiro.md` (cada quadro com o que foi
   dito enquanto estava na tela) e `dados.json`. O modelo de voz é baixado uma vez e conferido por SHA-256. O vocabulário
   privado (nomes de clientes e projetos dele) fica fora do repositório. Um vídeo sem faixa de áudio é processado só pela
   imagem.
3. **Ler.** Ler o roteiro inteiro e as folhas; abrir em tamanho cheio os quadros onde aparece uma janela, um menu ou uma
   tabela (as folhas reduzem a imagem e o texto pequeno some).
4. **Cruzar com o modelo.** Com o projeto do vídeo aberto no QiBuilder e o conector ligado, consultar o que o vídeo mostra:
   os elementos do pavimento (`builder-search_elements`), as propriedades de uma peça que aparece numa tabela
   (`builder-get_element_properties`), as peças que o catálogo oferece para ela (`builder-get_available_part`, que também
   serve de fonte para o cadastro de peças). **Só consultas de leitura**: nunca as que processam, substituem peças ou rodam
   rotinas (`execute_*`, `save_*`), que mudariam o projeto dele.
5. **Escrever a ficha** (`<vídeo> - ficha.md`, na pasta do Drive dele):
   - *Resumo* em duas linhas: a etapa do projeto e o objetivo.
   - *Passo a passo*: tempo, o que ele faz, o comando e onde fica (aba > painel), a janela que abre e as opções escolhidas.
   - *Dores*: o que ele diz que demora ou incomoda, com a citação e o tempo.
   - *Dados do modelo*: o que o conector confirmou ou acrescentou.
   - *Requisitos para o nosso programa*: o que a etapa pede, já em termos de recurso.
   - *Perguntas para o Jimmy*: o que ficou ambíguo, para ele responder por texto ou no vídeo seguinte.
6. **Consolidar.** Atualizar o índice (`00 - índice.md` na pasta do Drive: todos os vídeos, a etapa, o estado da análise) e,
   neste repositório, o catálogo de comandos e os requisitos em [docs/hidrossanitario.md](../../docs/hidrossanitario.md), sem
   nomes de clientes nem caminhos: o repositório é público.

## O que pedir nas gravações

Vídeos de até ~5 minutos ou ~100 MB, exportados em 720p; uma etapa por vídeo; a câmera num canto que não cubra painéis; zoom
nas janelas de diálogo; dizer o nome do comando ao usá-lo e por que o usa; dizer quando algo demora ou incomoda; conferir que o
áudio foi gravado.
