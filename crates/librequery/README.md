# librequery

Biblioteca do LibreBI que descreve uma consulta como **fonte + lista de passos** e
compila tudo em **um único texto SQL**, com uma CTE por passo. Nesta versão ela
não executa SQL nem lê dados: só modelo e geração de SQL.

## API

- `Query { id, source, steps }` — a consulta.
- `Step` — os passos: `SelectColumns`, `RemoveColumns`, `RenameColumns`,
  `ChangeType`, `Filter`, `AddColumn`, `Sort`, `GroupBy`, `Join`, `Append`,
  `Distinct`, `ReplaceValues` e `RawSql`.
- `compile(&Query) -> Result<String, CompileError>` — gera o SQL.
- `sql::quote_ident` e `sql::quote_literal` — citação de identificadores e literais.

`CompileError` cobre fonte vazia (`EmptySource`), lista de colunas vazia
(`EmptyColumns`), `GroupBy` sem chaves e sem agregações (`EmptyGroupBy`) e
identificador vazio (`EmptyIdentifier`). Uma consulta sem passos é válida e gera
apenas o CTE `s0`.

## Exemplo

```rust
use librequery::{compile, DataType, Query, Step};

let query = Query {
    id: "vendas".to_string(),
    source: "src_arquivo_vendas".to_string(),
    steps: vec![
        Step::SelectColumns(vec![
            "data".to_string(),
            "qtd".to_string(),
            "preco".to_string(),
        ]),
        Step::ChangeType { column: "data".to_string(), to: DataType::Date },
        Step::AddColumn { name: "valor".to_string(), expr: "qtd * preco".to_string() },
        Step::Filter("qtd > 0".to_string()),
    ],
};

let sql = compile(&query).unwrap();
```

Saída:

```sql
WITH s0 AS (SELECT * FROM "src_arquivo_vendas"),
     s1 AS (SELECT "data", "qtd", "preco" FROM s0),
     s2 AS (SELECT * EXCEPT ("data"), CAST("data" AS DATE) AS "data" FROM s1),
     s3 AS (SELECT *, qtd * preco AS "valor" FROM s2),
     s4 AS (SELECT * FROM s3 WHERE qtd > 0)
SELECT * FROM s4
```

## Dialeto

Escolha própria, simples e portável, documentada no topo de `src/compile.rs`:
identificadores entre aspas duplas (aspas internas dobradas), literais entre
apóstrofos (apóstrofos dobrados), `SELECT * EXCEPT (...)`, `CAST`, `FULL OUTER
JOIN`, `COUNT(DISTINCT ...)` e `UNION ALL`. Expressões em `Filter`,
`AddColumn::expr` e `RawSql` entram no SQL como texto do usuário; em `RawSql` o
identificador `prev` aponta para o passo anterior.
