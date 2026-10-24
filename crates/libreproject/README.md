# libreproject

Lê, grava e valida um projeto do LibreBI como pasta de texto versionável em git, e registra fontes e consultas no motor local. Sem interface gráfica.

## Layout

```text
meu-projeto/
├─ project.yaml          # name, format_version
├─ sources/<id>.yaml     # id, kind (csv | parquet), path relativo
└─ queries/<id>.yaml     # id, source, steps
```

Os passos usam `op` em *snake_case* (`select_columns`, `remove_columns`, `rename_columns`, `change_type`, `filter`, `add_column`, `sort`, `group_by`, `join`, `append`, `distinct`, `replace_values`, `raw_sql`). Tipos de dado: `int`, `float`, `decimal`, `bool`, `text`, `date`, `datetime`, `duration`.

A gravação é canônica: campos em ordem fixa, fontes e consultas ordenadas por id, indentação de dois espaços e terminador `\n`. Salvar duas vezes seguidas produz arquivos idênticos. Arquivos `.yaml` de `sources/` e `queries/` que não pertencem mais ao projeto são removidos; outros arquivos são preservados. Ids não podem ser vazios, `.`, `..` nem conter `/` ou `\`, pois viram nome de arquivo. O `path` de uma fonte precisa ser relativo à pasta do projeto e ficar dentro dela: caminhos absolutos, com `..`, vazios ou com byte nulo são recusados com `ProjectError::UnsafePath`, e `open_in` também recusa links simbólicos que escapem da pasta.

## API

- `Project::load(dir)` — lê, converte e valida os YAML, migrando a versão de formato (`FORMAT_VERSION = 1`);
- `Project::save(dir)` — valida e grava de forma canônica;
- `Project::validate()` — ids únicos, caminhos de fontes seguros, referências existentes e ausência de ciclos;
- `Project::query_order()` — consultas em ordem topológica, dependências primeiro;
- `Project::open_in(engine, dir)` — registra as fontes (conferindo o confinamento do caminho) e depois as consultas no motor;
- `ProjectError` — erros de E/S, YAML, versão, id, caminho inseguro, referência, ciclo e motor.

## Exemplo

```rust,no_run
use libreengine::{Engine, format_batches};
use libreproject::Project;

# async fn exemplo() -> Result<(), Box<dyn std::error::Error>> {
let dir = std::path::Path::new("examples/vendas");
let project = Project::load(dir)?;
project.validate()?;

let engine = Engine::new();
project.open_in(&engine, dir).await?;

let vendas = project
    .queries
    .iter()
    .find(|query| query.id == "vendas")
    .expect("consulta do exemplo");
let batches = engine.run(vendas).await?;
println!("{}", format_batches(&batches));
# Ok(())
# }
```

O projeto pronto em `examples/vendas/` tem as fontes `arquivo_vendas` e `produtos` e as consultas `vendas` e `receita_por_categoria`.

Os testes de integração usam `tokio` como *dev-dependency* porque `Engine` e `Project::open_in` são assíncronos. A pasta temporária vem de `tempfile`.
