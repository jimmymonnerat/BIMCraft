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

   Ela escreve, por vídeo: a transcrição (`.txt` e legenda `.srt`, cortada em frases pelo tempo de cada palavra), os
   quadros-chave (um depois de cada mudança de tela, já assentada, e pelo menos um a cada 15 s), folhas de quadros com o
   tempo marcado, o `roteiro.md` (cada quadro com o que foi dito enquanto estava na tela) e `dados.json`. O modelo de voz é
   baixado uma vez e conferido por SHA-256. O vocabulário e as correções privadas (nomes de clientes, projetos e pessoas)
   ficam fora do repositório (`--vocabulario` e `--correcoes` aceitam mais de um arquivo). Um vídeo sem faixa de áudio é
   processado só pela imagem. `--sem-quadros` refaz só a fala, mantendo os quadros de uma análise anterior.
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
6. **Consolidar.** Na pasta do Drive: o índice (`00 - índice.md`: todos os vídeos, a etapa, o estado da análise, os
   requisitos R1, R2…), a transcrição ao lado de cada vídeo e, em `analises/<vídeo>/`, o roteiro, as folhas e os quadros em
   `quadros.zip`, para a próxima conversa rever a tela sem reprocessar o vídeo. Neste repositório: o catálogo de comandos e
   os requisitos em [docs/hidrossanitario.md](../../docs/hidrossanitario.md), sem nomes de clientes, de produtos do cadastro
   nem caminhos: o repositório é público.

## Série 2: um vídeo, um ciclo

Desde a série 2 cada vídeo é uma etapa de um projeto feito nos dois programas: o vídeo é analisado e o programa já é preparado
para a mesma etapa. Para o ciclo não pesar:

1. **Uma conversa por vídeo.** O que importa fica fora da conversa: o código e os registros neste fork; os relatórios e o
   índice na pasta do projeto, no Drive dele; o vocabulário e as correções com nomes de clientes numa pasta `_privado` do
   Drive. Uma conversa nova começa por eles e não refaz nada.
2. **Numa máquina nova**, uma vez: `sh tools/videos/preparar.sh` (pacotes e modelos de voz). A fala:

   ```sh
   python3 -I tools/videos/transcrever.py "VIDEO.mp4" --saida ANALISES --vocabulario-frase FRASE-PRIVADA.txt \
       --correcoes tools/videos/correcoes.txt --correcoes CORRECOES-PRIVADAS.txt
   ```

   é o Whisper large-v3-turbo com o vocabulário e a fala anterior como contexto e as legendas cortadas por frase;
   `--so-legendas` refaz as legendas em segundos depois de uma correção nova. `montar_saida_projeto.py` monta a pasta do
   Drive: a legenda com o nome exato do vídeo, a transcrição, o roteiro, as folhas, os quadros e a fala no fim do relatório.
3. **O modelo pelo conector**: só o que o vídeo mostra de novo, e só leitura.
4. **Um relatório por vídeo**: passo a passo, como ele projeta, queixas, *no nosso programa: como fazer o mesmo* e perguntas;
   o índice ganha uma linha.
5. **Um lote de mudanças por vídeo**, teste primeiro, rodando aqui só os testes do que mudou (`cargo test -p CRATE FILTRO`,
   com `CARGO_INCREMENTAL=0` e um cargo por vez: a máquina tem dois núcleos). Um commit por passo e push logo: o GitHub roda
   a verificação completa (formatação, catálogos, clippy e todos os testes, em Linux, Windows e macOS) e gera o instalador
   Windows. O ciclo fecha com tudo verde.
6. **Olhar o resultado** só onde mudou, com o arquivo do cliente fora do repositório.

## O que pedir nas gravações

Vídeos de até ~5 minutos ou ~100 MB, exportados em 720p; uma etapa por vídeo; a câmera num canto que não cubra painéis; zoom
nas janelas de diálogo; dizer o nome do comando ao usá-lo e por que o usa; dizer quando algo demora ou incomoda; conferir que o
áudio foi gravado.
