# Hidrossanitário: registro do projeto

Registro vivo das decisões, do que já foi feito e do que falta no programa hidrossanitário do Jimmy (fork
`jimmymonnerat/BIMCraft`, branch `hidrossanitario`). Atualizar a cada etapa. Os detalhes técnicos de cada recurso estão nos
`docs/areas/*.md` citados; aqui fica o porquê e a visão de conjunto.

## Objetivo

Um programa desktop próprio para projetos hidrossanitários, no lugar do AltoQi QiBuilder (usado há mais de 20 anos). Do
QiBuilder ele usa **lançamento, detalhamento, geração de pranchas e lista de materiais**; não usa o dimensionamento. A
interface deve ficar o mais parecida possível com a do QiBuilder (organização das abas e comandos, nomes em português, fluxo),
sem copiar ícones, textos ou conteúdo da AltoQi.

## Decisões

| Quando | Decisão | Por quê |
|---|---|---|
| 30/10/2026 | Desktop, base de código aberto | Não começar do zero |
| 30/10/2026 | FreeCAD (bancada Python) → trocado pelo **BIMCraft** (Rust, egui, MIT/Apache) | Fluxo tipo Revit, IFC nativo, já traz NBR 5626/8160/10844, pode ser comandado por IA (MCP) |
| 30/10/2026 | Fork `jimmymonnerat/BIMCraft`, trabalho na branch `hidrossanitario` | `main` intocado: dá para voltar atrás; cada passo é um commit |
| 30/10/2026 | Atualizações do programa instalado vêm do fork, não do original | Para não trocar a versão dele pela original |
| 31/10/2026 | Só a aba Hidrossanitário na faixa de opções; Estrutura, Elétrica, SPDA e Dados ocultas (Configurações > Disciplinas) | Ele trabalha só com hidrossanitário. O código fica: é pequeno, é a mesma base dos tubos, e o leitor de IFC precisa dele para vigas, pilares etc. |
| 31/10/2026 | Vídeos do QiBuilder + conector MCP AltoQi como forma de mostrar o fluxo | Ver o trabalho real em vez de adivinhar por manuais |

## Fluxo do QiBuilder a reproduzir

1. **Criar projeto**: nome, pasta, nº de pavimentos e altura de cada um.
2. **Importar o IFC da arquitetura.**
3. **Definir escala** por dois pontos (faces de parede de 15 cm).
4. **Definir origem** comum a todas as disciplinas (compatibilização).
5. **Planificar**: o IFC cortado em linhas 2D para traçar por cima. O essencial são os **pontos hidrossanitários** (louças,
   cubas, torneiras, chuveiros, ralos).
6. Lançamento (água fria, quente, esgoto, ventilação, pluvial). *Ainda a gravar em vídeo.*
7. Detalhamento e **pranchas** (vídeo 1, abaixo).
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
| Louças do Revit lidas do IFC (IfcFlowTerminal do IFC2x3 e IfcDistributionElement, como o Revit exporta famílias Deca) | importação de IFC | — |
| Níveis da arquitetura a poucos cm um do outro aceitos | Criar projeto | — |
| Instalador Windows a cada envio à branch | GitHub Actions > "Windows (hidrossanitário)" > Artifacts | — |

## Na fila

1. **Pranchas como no QiBuilder** (prioridade, ver vídeos 1 e 2): região de detalhe, isométrico com ø/L e marcadores de peça
   ligados à tabela de peças, vista de parede do ponto, detalhes vivos na prancha, margem de modelo, pranchas HID/SAN.
2. **Lançamento** como no QiBuilder (depende dos vídeos dessa etapa).
3. **Cadastro de peças** a partir do banco .AQ do QiBuilder (ou lido pelo conector MCP AltoQi). O cadastro fica fora do
   repositório público.
4. **Interface parecida com o QiBuilder**: abas e painéis, nomes, atalhos. O manual instalado
   (`C:\Program Files\AltoQi\Builder\Ajuda\QiBuilder`) pode ser enviado como .zip ou a pasta conectada à conversa.
5. Conexões que ainda faltam: redução, curvas que não sejam de 90°, mudança de direção em prumada, cruzeta.
6. Nome próprio do programa (hoje aparece "BIMCraft").

## Análise dos vídeos

**O que ele disse nos vídeos 1 e 2** (transcrições na pasta de vídeos do Drive dele, um .txt por vídeo):
- A **geração de pranchas e o detalhamento são o principal gargalo** e o que ele mais quer **automatizar** no programa novo.
- Estimou **cerca de 5 horas** só organizando 13 pranchas (6 HID, 7 SAN) para impressão num projeto.
- O que toma tempo: cada "Inserir do projeto" demora a carregar; os detalhes não cabem e ele reorganiza à mão (o que não
  cabe vai para uma prancha nova); os detalhes ficam **desalinhados** e arrumar a prancha "harmônica aos olhos" é trabalho à
  parte; achar e ocultar um tubo cortado que aparece como "bolinha" num corte levou ~3 minutos (abrir original > vista 3D >
  ocultar > atualizar pranchas).
- Ordem dos detalhes por número (H10, H11, …), em sequência pela prancha; detalhes que "casam" (24 com 25) ficam lado a lado.
- No isométrico indica cada peça e põe a legenda de peças ao lado; as **alturas ele define no corte**.

**Consequência para a fila**: a prioridade das pranchas é **montagem automática**: o programa distribui os detalhes de um
pavimento em pranchas com a margem dele, em ordem, alinhados em grade, abrindo prancha nova quando não cabe, e deixa só o
ajuste fino para ele. Tubos cortados que só viram um ponto num corte devem sumir sozinhos.

**Vídeo 1** (31/10/2026, 4 min, um projeto real dele, etapa de pranchas):
- Na planta, regiões de detalhe marcadas com triângulos vermelhos de corte em volta de cada ambiente molhado (H21, H24, H25…).
- **Detalhe isométrico** por região: isométrico + tabela de peças ao lado + vistas das paredes.
- Árvore **Pranchas**: Desenhos > Hidráulico (HID 01–06), Sanitário (SAN 01–07), Arquitetônico; Margens (o modelo de
  margem com o carimbo da empresa dele); Revisões.
- Prancha nova: **Inserir margem** (escolhe o modelo) → **Inserir do projeto** (árvore: pavimento > Hidráulico/Sanitário/Drawing,
  Imagens 3D, Esquemas: isométrico hidráulico, vertical hidráulico, vertical sanitário) → arrasta os detalhes para dentro da margem.
- Faixa de opções usada: Arquivo, Desenhos, Elementos, Lançamento, Operações, Colaboração. Operações: Definir peças pendentes,
  Definir peças, Gerenciador de peças, Processar, Dimensionar condutos, Verificar diâmetros, Lista de materiais, Legenda de
  condutos, Legenda de peças, Detalhe isométrico, Inserir detalhe, Região de impressão, Gerenciador, Exibir croqui, Exibir
  detalhamento, Exibir vista 3D, Ocultar, Otimizar indicações. Elementos: Preencher área, Criar símbolo, Inserir símbolos,
  Inserir arquivo, Cota ortogonal, Linha de cota alinhada, Indicador, Emendar, Texto, Nuvem de revisão, Tabela de revisão.

**Vídeo 2** (31/10/2026, 5 min, mesmo projeto, montando a prancha HID 06):
- **Inserir do projeto** traz para a prancha os detalhes das regiões H26, H27 e H28 (banheiro BH da copa): o isométrico, a
  tabela de peças e as vistas das paredes.
- O **isométrico** tem cada tubo cotado com diâmetro e comprimento ("ø25 L=1.1") e cada peça com um marcador numerado (F1…F11)
  que remete à linha da **tabela de peças** ao lado. A tabela agrupa por material/tipo: "PVC rígido soldável – Joelho de
  redução 90 soldável 32 mm – 25 mm", "Aparelho – Ducha higiênica 25 mm × 1/2"", "Aparelho – Vaso sanitário c/ cx. acoplada
  1/2"", "PVC Acessórios – Engate flexível cobre cromado com canopla 1/2 – 30 cm", registro de gaveta (RG 1").
- A **vista da parede** (elevação do ponto) mostra as alturas cotadas (100, 120 cm) e os trechos verticais com ø e L.
- **Botão direito num detalhe na prancha**: Abrir original, Editar na prancha (com "Concluir edição"), Apagar, Mover,
  Rotacionar, Espelhar, Copiar, Escalar, Desagrupar, Trazer para frente, Enviar para trás, e comandos de nível (Definir nível
  atual, Desligar, Travar, Isolar).
- **Abrir original** abre o detalhe numa aba própria ("Pavimento 1 – Hidráulico – detalhe H27"); **Exibir vista 3D** mostra o 3D
  só daquele detalhe ("Visão 3D (Detalhe H27)"), com a aba 3D: Enquadrar 3D, Atualização parcial do 3D, Ocultar, Vistas,
  Configurações, Iluminação, Cor do corte, Perspectiva.
- Ao voltar à prancha, o QiBuilder **atualiza as pranchas sozinho** ("Atualizando pranchas: abrindo desenhos a serem
  atualizados (HID 06)… atualizando detalhamentos 25 %… 75 %"). **Os detalhes na prancha são vistas vivas do modelo**: mudou o
  lançamento, o detalhe e a tabela se refazem.

**O que isso pede do nosso programa** (pranchas):
1. Região de detalhe na planta (os triângulos de corte), com nome (H26…).
2. Detalhe isométrico da região: tubos com ø e L, peças com marcadores numerados, ligados a uma tabela de peças agrupada.
3. Vista da parede de cada ponto, com as alturas cotadas.
4. Detalhes como vistas vivas na prancha, refeitos quando o modelo muda; abrir o original, editar na prancha, mover, girar,
   escalar.
5. Margem/carimbo de modelo (o dele) e árvore de pranchas por disciplina (HID, SAN).

O BIMCraft já tem pranchas, vistas que se refazem do modelo (viewports), cortes, tabelas e 3D: a base existe; falta o detalhe
isométrico por região, os marcadores de peça ligados à tabela e a vista de parede do ponto.

**Como gravar os próximos**: Recordly, exportar em 720p e qualidade média (até ~100 MB por arquivo; dividir os longos);
câmera num canto que não cubra o painel Pranchas nem o carimbo; dar zoom nas janelas de diálogo; conferir que a gravação
tem áudio (um dos vídeos saiu sem); narrar o que faz e por quê. Os vídeos ficam numa pasta do Drive dele, que a sessão lê pelo
computador ligado; o processo de análise está em [tools/videos/README.md](../tools/videos/README.md).

## Referências do usuário

Os nomes dos projetos, dos clientes e os caminhos das pastas dele ficam fora deste repositório, que é público: estão nas
notas da sessão e na pasta de vídeos do Drive dele.
