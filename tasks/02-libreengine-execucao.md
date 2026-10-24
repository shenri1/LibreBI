# Tarefa 02 — LibreEngine: executar consultas do LibreQuery

Fonte de requisitos: apenas `docs/` (leia `AGENTS.md`; depois `docs/00-especificacao-mvp.md` seções 4 a 6 e
`docs/produto/02-roadmap.md`, fase 2). Não consulte nada fora deste diretório de trabalho.

## Objetivo
Criar o crate **libreengine**: um motor local que registra fontes de dados (CSV e Parquet), recebe uma `librequery::Query`,
compila para SQL com `librequery::compile` e **executa** o resultado, devolvendo linhas em formato Arrow. Inclui pré-visualização
limitada e pré-visualização "até o passo N" (resultado parcial de uma consulta). Sem interface gráfica nesta tarefa.

## Passos
1. Adicione `crates/libreengine` ao workspace (`cargo new --lib`, sem criar outro git). Dependências: `librequery` (caminho local),
   `datafusion` (use `cargo add datafusion` para a versão mais recente) e `tokio` com os recursos `rt` e `macros`.
   Nada além disso, exceto o necessário para os testes.
2. API pública (módulos pequenos e claros):
   - `Engine::new() -> Engine`.
   - `async fn register_csv(&self, name: &str, path: &Path) -> Result<(), EngineError>` (com cabeçalho e inferência de tipos).
   - `async fn register_parquet(&self, name: &str, path: &Path) -> Result<(), EngineError>`.
   - `async fn register_query(&self, query: &Query) -> Result<(), EngineError>`: registra o resultado da consulta como uma
     visão com o nome `query.id`, para que outras consultas a usem como fonte, em `Join` e em `Append`.
   - `async fn run(&self, query: &Query) -> Result<Vec<RecordBatch>, EngineError>`.
   - `async fn preview(&self, query: &Query, limit: usize) -> Result<Vec<RecordBatch>, EngineError>`: no máximo `limit` linhas.
   - `async fn preview_until_step(&self, query: &Query, step_index: usize, limit: usize) -> Result<Vec<RecordBatch>, EngineError>`:
     executa só os passos de 0 até `step_index` (inclusive). Erro claro se o índice não existir.
   - `fn format_batches(batches: &[RecordBatch]) -> String`: tabela em texto (para testes e depuração).
   - `EngineError`: encapsule erros de compilação do LibreQuery, de dados (DataFusion/Arrow) e de E/S, implementando `Display` e `Error`.
3. Se algum SQL gerado pelo `librequery` não executar no DataFusion, ajuste `crates/librequery/src/compile.rs` (e os testes golden
   dele) para o dialeto que funciona e registre a mudança em um comentário no topo de `compile.rs`. Todos os testes do
   `librequery` devem continuar passando.
4. Crie dados de teste em `crates/libreengine/tests/data/`: `vendas.csv` (colunas data, produto_id, qtd, preco; pelo menos 8 linhas,
   incluindo uma linha com qtd igual a zero e datas em duas semanas diferentes) e `produtos.csv` (produto_id, nome, categoria).
5. Testes de integração em `crates/libreengine/tests/` (use `#[tokio::test]`), comparando a saída de `format_batches` com texto esperado:
   o exemplo da seção 6 da especificação (selecionar colunas, mudar tipo de data para date, coluna calculada valor = qtd * preco,
   filtrar qtd > 0) sobre `vendas.csv`; um teste para cada outro passo (remover, renomear, ordenar, agrupar com soma e contagem,
   join com `produtos.csv`, append, distinct, substituir valores, SQL livre); `preview`; `preview_until_step` (inclusive um índice
   inválido); uma consulta usando outra consulta como fonte via `register_query`.
6. Rode `cargo fmt`, `cargo test --workspace` e `cargo build` e corrija tudo. O binário `librebi` deve continuar compilando.
7. Escreva `crates/libreengine/README.md` curto em português: API e um exemplo completo.

## Restrições
Código simples e legível, sem `unsafe`. Não crie nada além do descrito. Não altere o binário `librebi` nesta tarefa.
Responda no final com 4 linhas: arquivos criados, quantidade de testes, resultado do cargo test, ajustes feitos no dialeto do librequery.
