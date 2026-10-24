# Tarefa 01 — LibreQuery: modelo de passos e compilação para SQL

Fonte de requisitos: apenas `docs/` (leia `AGENTS.md`, depois `docs/00-especificacao-mvp.md` seção 6 e
`docs/produto/02-roadmap.md`). Não consulte nada fora deste repositório.

## Objetivo
Criar a biblioteca **librequery**: o "tratador" de dados do LibreBI. Uma consulta é uma fonte mais uma lista de passos; a
biblioteca compila essa lista em **um único texto SQL** composto por uma cadeia de CTEs (um CTE por passo). Nesta tarefa
**não executa** SQL e **não lê** dados: só modelo e geração de SQL.

## Passos
1. Transforme o projeto em workspace Cargo: mantenha o pacote `librebi` na raiz e adicione `[workspace]` com membro
   `crates/librequery`. Crie `crates/librequery` com `cargo new --lib` (sem criar outro git). Sem dependências externas.
2. Em `crates/librequery/src/` crie módulos pequenos: `model.rs` (tipos), `compile.rs` (geração de SQL), `sql.rs` (utilitários
   de citação de identificadores e literais), e `lib.rs` reexportando a API pública.
3. Tipos em `model.rs`:
   - `Query { id: String, source: String, steps: Vec<Step> }`
   - `Direction { Asc, Desc }`
   - `DataType { Int, Float, Decimal, Bool, Text, Date, DateTime, Duration }`
   - `Aggregation { name: String, func: AggFunc, column: Option<String> }` com `AggFunc { Sum, Avg, Min, Max, Count, CountDistinct }`
   - `JoinKind { Inner, Left, Right, Full }`
   - `Step`:
     `SelectColumns(Vec<String>)`, `RemoveColumns(Vec<String>)`, `RenameColumns(Vec<(String, String)>)` (antigo, novo),
     `ChangeType { column: String, to: DataType }`, `Filter(String)` (expressão SQL),
     `AddColumn { name: String, expr: String }`, `Sort(Vec<(String, Direction)>)`,
     `GroupBy { keys: Vec<String>, aggregations: Vec<Aggregation> }`,
     `Join { kind: JoinKind, other: String, left_key: String, right_key: String }`,
     `Append { other: String }`, `Distinct`,
     `ReplaceValues { column: String, from: String, to: String }` (valores de texto literais),
     `RawSql(String)` (consulta livre que usa o nome `prev` para o passo anterior).
4. `compile.rs`: `pub fn compile(query: &Query) -> Result<String, CompileError>`.
   - O CTE inicial `s0` é `SELECT * FROM <fonte>`; cada passo gera `s1`, `s2`, ... lendo do anterior; o resultado final é
     `SELECT * FROM <último>`.
   - Fontes e nomes de coluna são identificadores: cite com aspas duplas, escapando aspas internas dobrando-as.
   - Mapeamentos (dialeto: uma escolha simples e portável, documente-a em comentário no topo de `compile.rs`):
     SelectColumns -> lista de colunas; RemoveColumns -> `SELECT * EXCEPT (cols)`;
     RenameColumns -> `SELECT * EXCEPT (antigo), antigo AS novo` (para cada renomeação);
     ChangeType -> `SELECT * EXCEPT (col), CAST(col AS tipo) AS col`; Filter -> `WHERE`; AddColumn -> `SELECT *, expr AS nome`;
     Sort -> `ORDER BY`; GroupBy -> `SELECT chaves, funcoes AS nome ... GROUP BY chaves`; Join -> `JOIN ... ON`;
     Append -> `UNION ALL`; Distinct -> `SELECT DISTINCT *`; ReplaceValues -> `CASE WHEN col = literal THEN literal ELSE col END`
     com literais citados com apóstrofos (escape dobrando apóstrofos); RawSql -> o texto fornecido com `prev` apontando para o passo anterior.
   - Tipos: Int=BIGINT, Float=DOUBLE, Decimal=DECIMAL(38,9), Bool=BOOLEAN, Text=VARCHAR, Date=DATE, DateTime=TIMESTAMP, Duration=INTERVAL.
   - `CompileError` com variantes para: lista de passos vazia é permitida (gera só `s0`), colunas vazias em SelectColumns/RemoveColumns,
     GroupBy sem agregações e sem chaves, nome de fonte vazio, identificador vazio.
5. Testes (em `crates/librequery/tests/` ou `#[cfg(test)]`), estilo golden: compare o SQL gerado com o texto esperado.
   Inclua obrigatoriamente o exemplo da seção 6 do documento (select_columns, change_type data para date, add_column valor como
   qtd * preco, filter qtd > 0) com a fonte `src_arquivo_vendas`, e um teste por cada outro tipo de passo, mais testes dos erros
   e do escape de aspas.
6. Rode `cargo fmt`, `cargo test --workspace` e `cargo build` e corrija tudo. O binário `librebi` deve continuar compilando.
7. Escreva `crates/librequery/README.md` curto em português com a API e um exemplo.

## Restrições
Sem dependências externas. Sem unsafe. Código simples e legível. Não crie nada além do descrito.
Responda no final com 4 linhas: arquivos criados, quantidade de testes, resultado do cargo test, decisões de dialeto tomadas.
