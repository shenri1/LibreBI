# libreengine

Motor local do LibreBI. Registra fontes de dados (CSV e Parquet) em uma sessão do
[DataFusion](https://datafusion.apache.org/), compila consultas do `librequery`
com `librequery::compile` e executa o SQL, devolvendo linhas no formato Arrow
(`Vec<RecordBatch>`). Não há interface gráfica aqui.

## API

- `Engine::new()` — cria o motor com uma sessão vazia.
- `register_csv(name, path)` — registra um CSV com cabeçalho, inferindo os tipos.
- `register_parquet(name, path)` — registra um Parquet.
- `register_query(&Query)` — executa a consulta e registra o resultado como uma
  tabela com o nome `query.id`; outras consultas a usam como fonte (inclusive em
  `Join` e `Append`).
- `run(&Query)` — executa a consulta por inteiro.
- `preview(&Query, limit)` — executa devolvendo no máximo `limit` linhas.
- `preview_until_step(&Query, step_index, limit)` — executa só os passos de `0`
  até `step_index` (inclusive); `EngineError::StepOutOfRange` se o passo não
  existir.
- `format_batches(&[RecordBatch])` — tabela de texto (cabeçalho e células
  separadas por ` | `), útil em testes e depuração.

`EngineError` cobre erros de compilação do LibreQuery, do DataFusion/Arrow, de
E/S e o índice de passo inválido; implementa `Display` e `Error`.

## Exemplo

```rust,no_run
use libreengine::{Engine, format_batches};
use librequery::{DataType, Query, Step};

# async fn exemplo() -> Result<(), libreengine::EngineError> {
let engine = Engine::new();
engine
    .register_csv("vendas", std::path::Path::new("vendas.csv"))
    .await?;

let query = Query {
    id: "vendas".to_string(),
    source: "vendas".to_string(),
    steps: vec![
        Step::SelectColumns(vec![
            "data".to_string(),
            "produto_id".to_string(),
            "qtd".to_string(),
            "preco".to_string(),
        ]),
        Step::ChangeType { column: "data".to_string(), to: DataType::Date },
        Step::AddColumn { name: "valor".to_string(), expr: "qtd * preco".to_string() },
        Step::Filter("qtd > 0".to_string()),
    ],
};

let batches = engine.run(&query).await?;
println!("{}", format_batches(&batches));
# Ok(())
# }
```

## Observações

- O `librequery` emite o último passo como `SELECT` externo, então um `Sort`
  final ordena o resultado. Uma ordenação pedida no meio da cadeia de passos não
  é garantida pelo motor.
- A pré-visualização aplica `LIMIT` sobre a consulta já compilada; nada é
  materializado além do necessário.
