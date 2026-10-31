# Hidrossanitário: registro do projeto

Registro vivo das decisões, do que já foi feito e do que falta no programa hidrossanitário do Jimmy (fork
`jimmymonnerat/BIMCraft`, branch `hidrossanitario`). Atualizar a cada etapa. Os detalhes técnicos de cada recurso estão nos
`docs/areas/*.md` citados; aqui fica o porquê e a visão de conjunto.

## Objetivo

Um programa desktop próprio para projetos hidrossanitários, no lugar do AltoQi QiBuilder (usado há mais de 20 anos). Do
QiBuilder ele usa **lançamento, detalhamento, geração de pranchas e lista de materiais**; não usa o dimensionamento.

**O objetivo principal é economizar o tempo dele, não reproduzir o QiBuilder.** Nas palavras dele (vídeo 6): "Eu não quero
que você faça um sistema igual a esse, mas eu quero que você veja a forma que eu projeto, que eu boto em prancha, para que
você consiga identificar as minhas dores aqui e propor algo que me economize tempo". A interface pode lembrar a do QiBuilder
(nomes em português, organização dos comandos) para facilitar a troca, sem copiar ícones, textos ou conteúdo da AltoQi; onde
o QiBuilder obriga a trabalho repetido, o programa faz sozinho.

## Decisões

| Quando | Decisão | Por quê |
|---|---|---|
| 30/10/2026 | Desktop, base de código aberto | Não começar do zero |
| 30/10/2026 | FreeCAD (bancada Python) → trocado pelo **BIMCraft** (Rust, egui, MIT/Apache) | Fluxo tipo Revit, IFC nativo, já traz NBR 5626/8160/10844, pode ser comandado por IA (MCP) |
| 30/10/2026 | Fork `jimmymonnerat/BIMCraft`, trabalho na branch `hidrossanitario` | `main` intocado: dá para voltar atrás; cada passo é um commit |
| 30/10/2026 | Atualizações do programa instalado vêm do fork, não do original | Para não trocar a versão dele pela original |
| 31/10/2026 | Só a aba Hidrossanitário na faixa de opções; Estrutura, Elétrica, SPDA e Dados ocultas (Configurações > Disciplinas) | Ele trabalha só com hidrossanitário. O código fica: é pequeno, é a mesma base dos tubos, e o leitor de IFC precisa dele para vigas, pilares etc. |
| 31/10/2026 | Vídeos do QiBuilder + conector MCP AltoQi como forma de mostrar o fluxo | Ver o trabalho real em vez de adivinhar por manuais |
| 31/10/2026 | **Prioridade: montar pranchas e detalhes sozinho** (distribuição, carimbo, indicação de peças), antes de parecer com o QiBuilder | Os sete vídeos mostram ~5 h para organizar 13 pranchas e esperas a cada passo; ele pediu economia de tempo |
| 31/10/2026 | **Um projeto novo feito nos dois programas, passo a passo**: ele grava cada etapa no QiBuilder, a análise prepara o programa para repetir a mesma etapa | "Todos os passos que eu fizer aqui, nós obrigatoriamente temos que conseguir fazer no meu aplicativo" (série 2, vídeo 1) |
| 31/10/2026 | **Os pavimentos são dele, não da arquitetura**: escolher o IFC no Criar projeto não muda mais os pavimentos | "Eu não quero depender da arquitetura… Não me interessa como o arquiteto considerou os pavimentos no projeto dele" (série 2, vídeo 1); a arquitetura do teste tinha nove níveis |
| 31/10/2026 | **A arquitetura vinculada só aparece no 3D** até ele pedir o plano de corte; depois a planta mostra a planificação | "Só vai existir algum elemento a partir do momento que eu clicar e solicitar o plano de corte" (série 2, vídeo 1) |

## Fluxo do QiBuilder a reproduzir

1. **Criar projeto**: nome, pasta, nº de pavimentos e altura de cada um, os dele (numa casa térrea: térreo e cobertura,
   2,80 m, "depois eu altero"); as pranchas organizadas por disciplina (Hidráulico, Sanitário), numeradas por disciplina.
2. **Vincular o IFC da arquitetura**: aparece só no 3D; as plantas do hidráulico e do sanitário ficam vazias.
3. **Definir escala** por dois pontos (faces de parede de 15 cm).
4. **Definir origem** comum a todas as disciplinas (compatibilização).
5. **Planificar** ("plano de corte"): uma janela diz, para cada pavimento, onde corta e até onde vê para baixo (no QiBuilder,
   1,68 m e −0,42 m num pavimento de 2,80 m); o IFC vira linhas 2D por pavimento para traçar por cima. Depois ele põe tudo numa
   cor só (cinza) e apaga a mobília que polui (sofá, tapete, piano, cortina). O essencial são os **pontos hidrossanitários**
   (louças, cubas, torneiras, chuveiros, ralos), e os **snaps** na base têm de estar sempre à mão ao lançar os tubos.
6. Lançamento (água fria, quente, esgoto, ventilação, pluvial). *Ainda a gravar em vídeo.*
7. Detalhamento e **pranchas** (vídeos 1 a 7, abaixo).
8. **Lista de materiais.**

## Já feito (branch `hidrossanitario`)

| Recurso | Onde | Comando |
|---|---|---|
| Gerar conexões (joelho 90°, tê) onde tubos se encontram; lista pontas abertas e o que não sabe conectar | aba Hidrossanitário > Rede de tubulação | `pipe_fittings` |
| Lista de materiais: tubos em m e barras de 6 m, peças contadas; .xlsx ou .csv | aba Hidrossanitário > Rede de tubulação | `material_list` |
| Criar projeto: nome, pasta, pavimentos; com o IFC escolhido, os pavimentos seguem os níveis dele e o IFC é vinculado | página inicial | `new` (heights, names), `levels_in` |
| Definir escala: DXF/DWG escalado por dois pontos; IFC só conferido | aba Hidrossanitário > Base | `scale_by_reference` |
| Definir origem: o ponto clicado vira 0,0, tudo junto | aba Hidrossanitário > Base | `set_origin` |
| Planificar: o IFC vinculado vira linhas 2D cinzas por pavimento, camada por categoria, sem hachuras | aba Hidrossanitário > Base | `flatten_link` |
| Criar projeto com os pavimentos dele (térreo 2,80 m e cobertura, por padrão), não os da arquitetura; mostra o nível de cada pavimento; o IFC escolhido fica só no 3D | página inicial | `new`, `link` (`in_plans: false`) |
| Plano de corte: janela com cada pavimento, corte e limite inferior (padrão 60 % e −15 % da altura, como o QiBuilder), desenho explicativo; o modelo continua no 3D | aba Hidrossanitário > Base > Planificar | `flatten_link` (`levels: [{level, cut, bottom}]`, `keep_link`) |
| Planificação com a qualidade do QiBuilder: louças e móveis desenhados como vistos de cima (bordas, cubas, assentos), não pelo contorno convexo | planificação e plantas | — |
| Base numa cor só (cinza), cada elemento um objeto, camadas pelo que é (parede, porta, mobiliário, louças…), ralos como círculos (snap no centro) | planificação | — |
| Apagar da base: um clique tira um objeto inteiro (sofá, tapete, piano); fica apagado se planificar de novo; Ctrl+Z desfaz | aba Hidrossanitário > Base | `cad_erase` |
| Louças do Revit lidas do IFC (IfcFlowTerminal do IFC2x3 e IfcDistributionElement, como o Revit exporta famílias Deca) | importação de IFC | — |
| Níveis da arquitetura a poucos cm um do outro aceitos | Criar projeto | — |
| Instalador Windows a cada envio à branch | GitHub Actions > "Windows (hidrossanitário)" > Artifacts | — |
| Análise dos vídeos: transcrição, quadros-chave e roteiro de cada gravação | `tools/videos` ([processo](../tools/videos/README.md)) | — |

## Na fila

1. **Montar pranchas** (prioridade): a proposta e os requisitos R1 a R8 estão abaixo.
2. **Lançamento** como no QiBuilder (depende dos vídeos dessa etapa).
3. **Cadastro de peças** (R9) a partir do banco .AQ do QiBuilder ou lido pelo conector MCP AltoQi
   (`builder-get_available_part`). O cadastro fica fora do repositório público.
4. **Padrões de desenho do escritório** (R10).
5. **Interface parecida com o QiBuilder**: abas e painéis, nomes, atalhos. O manual instalado
   (`C:\Program Files\AltoQi\Builder\Ajuda\QiBuilder`) pode ser enviado como .zip ou a pasta conectada à conversa.
6. Conexões que ainda faltam: redução, curvas que não sejam de 90°, mudança de direção em prumada, cruzeta.
7. Nome próprio do programa (hoje aparece "BIMCraft").

## Análise dos vídeos

Sete vídeos gravados em 31/10/2026, todos de um mesmo projeto real dele, na etapa final (detalhamento e pranchas). As fichas
completas (passo a passo com tempos, citações, perguntas), as transcrições e os quadros ficam na pasta de vídeos do Drive
dele, com um índice; o processo está em [tools/videos/README.md](../tools/videos/README.md).

| # | Duração | Etapa | O que mostra |
|---|---|---|---|
| 1 | 4:07 | Pranchas do hidráulico | Detalhes inseridos um a um na ordem dos números e rearrumados à mão; o que não coube vai para prancha nova; ele perde a conta de qual detalhe sobrou |
| 2 | 4:51 | Pranchas do hidráulico | Isométrico e corte por ambiente; um tubo cortado aparece como "bolinha" no corte e leva ~3 min para ocultar; estimativa de 5 h para 13 pranchas |
| 3 | 4:07, sem áudio | Detalhe da cisterna | Propriedades do detalhe isométrico; configurações de desenho do hidráulico; seis esperas de "Gerando modelo 3D" em 4 min |
| 4 | 5:23 | Detalhe dos pressurizadores | Ocultar a caixa só no detalhe; perfil de cores dele; indicação de peças por conjunto; legenda de peças; otimizar indicações |
| 5 | 5:04 | Detalhes e planta | Detalhes levados de uma prancha para outra quase vazia; texto do reservatório digitado (câmaras, dimensões, volume, reserva de incêndio) |
| 6 | 4:37 | Carimbo do hidráulico | Dados do projeto alimentam o carimbo de todas as pranchas; conteúdo e disciplina digitados prancha a prancha; centralizar à mão |
| 7 | 2:46 | Carimbo do sanitário | O mesmo no sanitário: planta do térreo, do superior, de cobertura, detalhes |

**O que os vídeos mostram**:
- A **geração de pranchas e o detalhamento são o principal gargalo**: ele estima **cerca de 5 horas** só organizando 13
  pranchas (6 HID, 7 SAN) para impressão.
- **Esperas o tempo todo**: 15 a 20 s a cada "Inserir do projeto", uma janela "Gerando modelo 3D" a cada mudança num detalhe,
  "Atualizar pranchas" para ver qualquer correção. "Demora bastante, como qualquer procedimento dentro do QiBuilder, que é
  uma coisa que eu não quero que o nosso programa aconteça" (vídeo 4).
- **Trabalho repetido à mão**: arrumar, alinhar e centralizar detalhes; juntar pranchas vazias; digitar conteúdo e disciplina
  em cada prancha; escolher o perfil de cores detalhe a detalhe; apagar textos automáticos (e, por engano, números);
  digitar dimensões e volume do reservatório; achar e ocultar tubos cortados.
- **O que funciona e deve ficar**: os dados do projeto, preenchidos uma vez, atualizam o carimbo de todas as pranchas; a
  numeração "folha n / N" é automática; os detalhes na prancha são vistas vivas do modelo.

### Requisitos

| | Requisito | Vídeos |
|---|---|---|
| R1 | **Montagem automática das pranchas**: plantas primeiro, depois detalhes na ordem dos números; os que "casam" lado a lado; grade alinhada; centralizado na área útil; prancha nova quando não cabe; nenhuma prancha quase vazia | 1, 2, 5, 6, 7 |
| R2 | **Detalhes fora de prancha**: saber quais detalhes ainda não estão em nenhuma prancha e em qual está cada um | 1 |
| R3 | **Carimbo e propriedades automáticos**: dados do projeto uma vez; conteúdo (planta baixa do térreo / do pavimento superior / de cobertura / detalhes), disciplina (pela pasta HID ou SAN), folha n/N e data deduzidos; renumerar quando uma prancha sai; o carimbo do escritório como modelo | 5, 6, 7 |
| R4 | **Rapidez**: nada de janela de espera; detalhes e pranchas atualizados sozinhos; salvar em segundo plano | todos |
| R5 | **Detalhe isométrico por região com os padrões dele**: região com nome (H21…) desenhada na planta; vista 3D ou em linhas; perfil de cores dele; rotação 30°; escala 1:25; a altura abrangida calculada pelos elementos da região; ocultar um elemento só naquele detalhe | 3, 4 |
| R6 | **Indicação de peças e legenda automáticas**: marcadores F (fria), Q (quente), A (alimentação) numerados por detalhe; um marcador por conjunto de peças (válvula com os adaptadores, filtro com niples e luvas); legenda agrupada por material com a quantidade por marcador; texto 1,5, margens 0,5, espaçamento 50 %; posições sem sobreposição já na criação | 2, 4 |
| R7 | **Detalhes limpos**: tubo que atravessa o plano do corte e vira um ponto some sozinho; só os textos automáticos que ele quer; nada de apagar números por engano | 2, 3, 4 |
| R8 | **Anotação de reservatório** gerada do elemento: nome, câmara, dimensões, volume calculado, observação de reserva de incêndio | 5 |
| R9 | **Cadastro de peças** com o que ele usa: pressurizadores, filtro Y, válvula de esfera, niple duplo, luva soldável com bucha de latão, adaptador soldável curto para registro | 3, 4 |
| R10 | **Padrões de desenho do escritório**: prefixos F/Q/A; cores por sistema (água fria ciano, quente vermelho, alimentação verde); comprimento em m com 1 casa decimal; cota de elevação com prefixo "Ct" referida ao pavimento | 3 |

### Proposta: "Montar pranchas" num comando

1. Cada região de detalhe vira um detalhe pronto com os padrões dele (R5), marcadores e legenda já posicionados (R6),
   sem tubos cortados nem textos sobrando (R7).
2. O programa distribui tudo nas pranchas da disciplina com o carimbo dele (R1): plantas primeiro, detalhes em ordem, pares
   lado a lado, em grade, centralizados; abre prancha nova quando não cabe e não deixa prancha quase vazia.
3. Preenche o carimbo (R3) e lista o que ficou de fora (R2).
4. Deixa para ele só o ajuste fino; mudou o projeto, tudo se refaz sem "Atualizar pranchas" (R4).

O BIMCraft já tem boa parte da base:

| Parte | No BIMCraft hoje | Falta |
|---|---|---|
| Prancha com carimbo | `sheet`: A0 a A4, carimbo desenhado em código, números A101… | carimbo do escritório como modelo; numeração HID 01 / SAN 01; campos deduzidos (R3) |
| Detalhe isométrico | vista 3D com caixa de corte; `place` põe uma vista 3D na prancha em escala, com azimute e elevação | região de detalhe com nome que gera a vista recortada; padrões dele (R5) |
| Ocultar só no detalhe | Visibility/Graphics: `hide_elements` por vista | ocultar sozinho o tubo cortado (R7) |
| Marcadores de peça | `tag` (Tag by Category) | numeração F/Q/A por detalhe e por conjunto; posição sem sobreposição (R6) |
| Legenda de peças | `schedule` | tabela só das peças do detalhe, agrupada por material, com o marcador (R6) |
| Vistas na prancha | `place`, `crop`, `duplicate_view`, `scale` | distribuição automática (R1); detalhes fora de prancha (R2) |
| Anotação | `text` | texto do reservatório gerado do elemento (R8) |

### Catálogo: comandos do QiBuilder vistos e o equivalente

| QiBuilder (onde) | O que faz nos vídeos | No nosso programa |
|---|---|---|
| Arquivo > Novo (prancha, arquivo .CAD na pasta Hidráulico ou Sanitário) | prancha nova | `sheet`; a montagem cria as pranchas sozinha (R1) |
| Inserir margem | põe a margem com o carimbo do escritório | carimbo do escritório como modelo (R3) |
| Inserir do projeto | traz um detalhe ou planta para a prancha (15 a 20 s cada) | `place`, sem espera; montagem automática (R1) |
| Atualizar pranchas | refaz detalhes e textos nas pranchas | automático (R4) |
| Editar propriedades (prancha) | conteúdo, disciplina, data, nº da folha | deduzidos (R3) |
| Elementos > Acessar > Edificação (Propriedades da edificação) | dados do projeto que alimentam o carimbo de todas as pranchas | dados do projeto (R3) |
| Operações > Detalhe isométrico; Exibir croqui / detalhamento / vista 3D | região de detalhe e as representações (linhas, 3D) | R5 |
| Ocultar | esconde um elemento num detalhe | `hide_elements` |
| Filtro de seleção | seleciona por classe de peça, cor, nível, rede ou tipo de elemento | `filter_selection` |
| Lançamento > Pontos > Indicação peça | marcador F1… numa peça | `tag` + R6 |
| Operações > Tabelas > Legenda de peças | tabela das peças do detalhe | `schedule` + R6 |
| Operações > Otimizações > Otimizar indicações | rearruma os marcadores sem sobreposição | R6 |
| Elementos > Textos > Texto | anotação na planta | `text`; R8 |
| Arquivo > Configurações > Hidráulico > Padrões de desenho | prefixos das peças, cores, unidades, cotas | R10 |
| Abrir original / Editar na prancha | abre o detalhe numa aba própria / edita dentro da prancha | a ver |
| Copiar e colar entre pranchas; apagar prancha | junta pranchas | R1, R3 |

### Detalhes vistos

**Vídeo 1**:
- Na planta, regiões de detalhe marcadas com triângulos vermelhos de corte em volta de cada ambiente molhado (H21, H24, H25…).
- **Detalhe isométrico** por região: isométrico + tabela de peças ao lado + vistas das paredes.
- Árvore **Pranchas**: Desenhos > Hidráulico (HID 01–06), Sanitário (SAN 01–07), Arquitetônico; Margens (o modelo de
  margem com o carimbo da empresa dele); Revisões.
- Prancha nova: **Inserir margem** (escolhe o modelo) → **Inserir do projeto** (árvore: pavimento > Hidráulico/Sanitário/Drawing,
  Imagens 3D, Esquemas: isométrico hidráulico, vertical hidráulico, vertical sanitário) → arrasta os detalhes para dentro da margem.
- Ordem dos detalhes por número (H10, H11, …), em sequência pela prancha; detalhes que "casam" (24 com 25) ficam lado a lado.
- Faixa de opções usada: Arquivo, Desenhos, Elementos, Lançamento, Operações, Colaboração. Operações: Definir peças pendentes,
  Definir peças, Gerenciador de peças, Processar, Dimensionar condutos, Verificar diâmetros, Lista de materiais, Legenda de
  condutos, Legenda de peças, Detalhe isométrico, Inserir detalhe, Região de impressão, Gerenciador, Exibir croqui, Exibir
  detalhamento, Exibir vista 3D, Ocultar, Otimizar indicações. Elementos: Preencher área, Criar símbolo, Inserir símbolos,
  Inserir arquivo, Cota ortogonal, Linha de cota alinhada, Indicador, Emendar, Texto, Nuvem de revisão, Tabela de revisão.

**Vídeo 2**:
- **Inserir do projeto** traz para a prancha os detalhes das regiões H26, H27 e H28 (um banheiro): o isométrico, a tabela de
  peças e as vistas das paredes.
- O **isométrico** tem cada tubo cotado com diâmetro e comprimento ("ø25 L=1.1") e cada peça com um marcador numerado (F1…F11)
  que remete à linha da **tabela de peças** ao lado. A tabela agrupa por material/tipo: "PVC rígido soldável – Joelho de
  redução 90 soldável 32 mm – 25 mm", "Aparelho – Ducha higiênica 25 mm × 1/2"", "Aparelho – Vaso sanitário c/ cx. acoplada
  1/2"", "PVC Acessórios – Engate flexível cobre cromado com canopla 1/2 – 30 cm", registro de gaveta (RG 1").
- No isométrico ele indica cada peça; as **alturas ele define no corte**. A **vista da parede** (elevação do ponto) mostra as
  alturas cotadas (100, 120 cm) e os trechos verticais com ø e L.
- **Botão direito num detalhe na prancha**: Abrir original, Editar na prancha (com "Concluir edição"), Apagar, Mover,
  Rotacionar, Espelhar, Copiar, Escalar, Desagrupar, Trazer para frente, Enviar para trás, e comandos de nível (Definir nível
  atual, Desligar, Travar, Isolar).
- **Abrir original** abre o detalhe numa aba própria ("Pavimento 1 – Hidráulico – detalhe H27"); **Exibir vista 3D** mostra o 3D
  só daquele detalhe ("Visão 3D (Detalhe H27)"), com a aba 3D: Enquadrar 3D, Atualização parcial do 3D, Ocultar, Vistas,
  Configurações, Iluminação, Cor do corte, Perspectiva.
- Ao voltar à prancha, o QiBuilder **atualiza as pranchas** ("Atualizando pranchas: abrindo desenhos a serem atualizados
  (HID 06)… atualizando detalhamentos 25 %… 75 %"). **Os detalhes na prancha são vistas vivas do modelo**: mudou o
  lançamento, o detalhe e a tabela se refazem.
- Um tubo cortado que aparecia como "bolinha" num corte: Abrir original > Exibir vista 3D para achar o tubo > Ocultar >
  Atualizar pranchas. Cerca de 3 minutos.

**Vídeo 3** (propriedades do **detalhe isométrico**):
- Identificação: nome (H21), prefixo (H), número (21).
- Representação nas pranchas: Adotar configuração, Unifilar, Bifilar realista, Bifilar realista com modelo externo, Vista 3D,
  Vista 3D com modelo externo. Perfil de vista 3D: os da AltoQi ou um do usuário (ele tem um para as pranchas do hidráulico).
- Aplicar cores consistentes; extensão acima e abaixo do pavimento (ele subiu de 50 e 75 para 250 para o reservatório
  aparecer inteiro); rotação 30°; escala 1:25; estilo e alinhamento do título.
- **Configurações > Hidráulico > Padrões de desenho**: condutos entre pavimentos indicados na planta e no detalhe; condutos
  em linha dupla; comprimento com 1 casa decimal em m, sem zeros à direita; cores padrão por sistema; indicação das colunas
  pelo diâmetro; **prefixo da indicação das peças por sistema (F, Q, A)**; cotas de elevação referidas ao pavimento, com
  prefixo "Ct".

**Vídeo 4** (indicação de peças e **legenda de peças**):
- Indicação peça marca uma peça ou um conjunto; o mesmo marcador pode aparecer em peças iguais (F1 nas duas bombas).
- A legenda lista, por marcador, o grupo (Pressurizador, Metais, PVC rígido soldável…), a peça e a quantidade por
  marcador; propriedades: incluir referências externas, incluir elementos não visíveis, exibir numeração, hachura de
  destaque, pavimento de origem, exibir grupo de peça, fonte e altura do texto, margens e espaçamento.
- "Otimizar indicações" rearruma os marcadores depois que ele reduz a altura do texto (2 → 1,5).

**Vídeos 6 e 7** (carimbo):
- Propriedades da edificação (Identificação, Projetos, Pavimentos, Localização, Desenho, Pranchas): nome do empreendimento,
  tipo, categoria, iniciativa, descrição, localização, cliente, autor e registro profissional; alimentam todas as pranchas.
- Propriedades de cada prancha: área útil (ponto inicial, largura, altura), nome e pasta do arquivo, conteúdo da prancha,
  disciplina, projeto, pavimento, nº da folha e nº de folhas, nº do conjunto e de conjuntos.
- O carimbo dele tem: dados da empresa; responsável técnico e registro; nº da ART; áreas (existente, a construir, subsolo,
  térreo, total); escala (INDICADA); folha n/N; revisão; disciplina na vertical; proprietário; obra; endereço; conteúdo;
  data.

**Como gravar os próximos**: Recordly, exportar em 720p e qualidade média (até ~100 MB por arquivo; dividir os longos);
câmera num canto que não cubra o painel Pranchas nem o carimbo; dar zoom nas janelas de diálogo; conferir que a gravação
tem áudio (um dos vídeos saiu sem); narrar o que faz e por quê. Faltam: o **lançamento** (água fria, quente, esgoto,
ventilação, pluvial), **criar uma região de detalhe** do zero e a **lista de materiais** como ele entrega. Os vídeos ficam
numa pasta do Drive dele, que a sessão lê pelo computador ligado; o processo de análise está em
[tools/videos/README.md](../tools/videos/README.md).

## Série 2: um projeto do zero, nos dois programas

A partir de 31/10/2026 ele grava um projeto novo (uma residência térrea) do começo ao fim no QiBuilder, uma etapa por vídeo, e
o programa é preparado para repetir a mesma etapa. Os relatórios de cada vídeo ficam na pasta de vídeos do Drive dele; o
ciclo de cada vídeo está em [tools/videos](../tools/videos/README.md) ("Série 2: um vídeo, um ciclo").

| # | Duração | Etapa | O que pede ao programa | Feito |
|---|---|---|---|---|
| 1 | 5:00 | Criar o projeto, vincular o IFC, plano de corte | Pavimentos dele (térreo e cobertura, 2,80 m), não os da arquitetura; IFC só no 3D até o plano de corte; uma janela que diga onde corta e o que se vê em cima e embaixo, por pavimento | Criar projeto; vínculo só no 3D; Plano de corte |
| 2 | 3:01 | A planificação como base do projeto | Planificação com a mesma qualidade do QiBuilder; tudo numa cor só; tirar a mobília que não interessa; snaps sempre disponíveis ao lançar | Desenho visto de cima; cinza; Apagar da base; objetos e círculos (snap) |

**O QiBuilder nestes vídeos**: Arquivo > Novo > Nova edificação (6 passos: identificação e pasta; modelo; disciplinas;
propriedades de cálculo; pavimentos com nome, repetições, altura e nível; forma de organização das pranchas: pastas por projeto,
numeração por projeto). Árvore da edificação: cada pavimento com o desenho do Hidráulico e do Sanitário; Modelos 3D (o IFC);
Imagens 3D; Esquemas. Botão direito no IFC > Plano de corte: tabela por pavimento (seleção, altura, corte superior e inferior,
nível, nível superior e inferior) com um esquema do corte e o padrão de abrangência dos objetos; a planificação vira um desenho
por pavimento, camadas "IFC-Parede", "IFC-Mobiliário" etc. A cor única ele põe camada por camada em Configurações > Desenho >
Níveis de desenho (cor 251, cinza).

**Ainda não**: as plantas por disciplina (Hidráulico e Sanitário em cada pavimento) como na árvore do QiBuilder; a escolha do
que entra na planificação por categoria (o "padrão de abrangência"); esconder linhas de um objeto sob outro objeto (hoje só
dentro do mesmo objeto); o que o próximo vídeo mostrar (lançamento).

## Referências do usuário

Os nomes dos projetos, dos clientes e os caminhos das pastas dele ficam fora deste repositório, que é público: estão nas
notas da sessão e na pasta de vídeos do Drive dele.
