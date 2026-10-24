# Tarefa 03 — LibreProject: salvar e abrir projetos em pasta de texto (YAML)

Fonte de requisitos: apenas `docs/` (leia `AGENTS.md`; depois `docs/00-especificacao-mvp.md` seções 5, 6 e 10 e
`docs/produto/02-roadmap.md`). Não consulte nada fora deste diretório de trabalho.

## Objetivo
Criar o crate **libreproject**: leitura e gravação de um projeto como **pasta de texto versionável em git**, contendo fontes de
dados e consultas do LibreQuery. Também valida o projeto e o executa no `libreengine`. Sem interface gráfica nesta tarefa.

## Layout da pasta do projeto
```text
meu-projeto/
├─ project.yaml          # name, format_version (inteiro, versão atual = 1)
├─ sources/<id>.yaml     # id, kind (csv | parquet), path (relativo à pasta do projeto)
└─ queries/<id>.yaml     # id, source, steps
```
Formato dos passos em YAML (campo `op` em snake_case; exemplo da seção 6 da especificação):
```yaml
id: vendas
source: arquivo_vendas
steps:
  - op: select_columns
    columns: [data, qtd, preco]
  - op: change_type
    column: data
    to: date
  - op: add_column
    name: valor
    expr: "qtd * preco"
  - op: filter
    expr: "qtd > 0"
```
Cada tipo de passo do `librequery::Step` precisa de uma forma YAML com campos nomeados (`select_columns`, `remove_columns`,
`rename_columns` com lista de pares de e novo, `change_type`, `filter`, `add_column`, `sort` com lista de coluna e direção
asc ou desc, `group_by` com keys e aggregations, `join`, `append`, `distinct`, `replace_values`, `raw_sql`). Tipos de dado em minúsculas
(int, float, decimal, bool, text, date, datetime, duration).

## Passos
1. Adicione `crates/libreproject` ao workspace (`cargo new --lib`, sem criar outro git). Dependências: `librequery` e `libreengine`
   (caminhos locais), `serde` com `derive`, e uma biblioteca YAML **mantida** para serde (use `serde_yaml_ng`; NÃO use `serde_yaml`,
   que está arquivada). Em dev-dependencies, `tempfile`. Nada além disso.
2. NÃO altere o `librequery` para depender de serde: crie tipos próprios de formato em arquivo (DTOs com `Serialize`/`Deserialize`,
   `#[serde(tag = "op", rename_all = "snake_case")]` para os passos) e converta de e para `librequery::Step`/`Query`.
3. API pública:
   - `Project { name: String, format_version: u32, sources: Vec<SourceDef>, queries: Vec<Query> }` e `SourceDef { id, kind, path }`.
   - `Project::load(dir: &Path) -> Result<Project, ProjectError>` e `Project::save(&self, dir: &Path) -> Result<(), ProjectError>`.
   - `Project::validate(&self) -> Result<(), ProjectError>`: ids únicos (fontes e consultas, e sem colisão entre os dois conjuntos);
     o `source` de cada consulta e o `other` de `Join`/`Append` devem existir como fonte ou consulta; sem ciclos entre consultas.
   - `Project::query_order(&self) -> Result<Vec<&Query>, ProjectError>`: ordem topológica (dependências primeiro).
   - `async fn Project::open_in(&self, engine: &Engine, dir: &Path) -> Result<(), ProjectError>`: registra todas as fontes no motor e
     depois registra as consultas na ordem de dependência, para que `engine.run` funcione em qualquer consulta pelo seu id.
   - `ProjectError` com variantes claras (E/S com o caminho, YAML com o arquivo, versão de formato não suportada, id duplicado,
     referência inexistente, ciclo, motor), implementando `Display` e `Error`, mensagens em português.
4. Gravação **canônica**: ordem fixa de campos, listas de consultas e fontes ordenadas por id, indentação fixa e terminador de linha
   `\n`; salvar duas vezes seguidas deve produzir arquivos idênticos. Ao salvar, remova arquivos de `sources/` e `queries/` que não
   pertençam mais ao projeto.
5. Versão de formato: a atual é 1. Se `format_version` for maior que a suportada, erro claro; se menor, passe por uma função
   `migrate` (hoje não faz nada além de aceitar a versão 1) para que migrações futuras tenham onde entrar.
6. Crie o exemplo `examples/vendas/` na raiz do repositório: `project.yaml`, `sources/arquivo_vendas.yaml` (csv, `data/vendas.csv`),
   `sources/produtos.yaml`, `queries/vendas.yaml` (exemplo acima), `queries/receita_por_categoria.yaml` (usa `vendas` como fonte,
   faz join com `produtos` e agrupa somando valor por categoria), e `data/vendas.csv` e `data/produtos.csv` (copie de
   `crates/libreengine/tests/data/`).
7. Testes em `crates/libreproject/tests/`: ida e volta (salvar, carregar e comparar, para um projeto com todos os tipos de passo);
   saída canônica do exemplo da seção 6 comparada com o texto esperado; salvar duas vezes gera arquivos idênticos; remoção de arquivos
   obsoletos; cada erro de validação; YAML inválido; `op` desconhecido; versão de formato maior que a suportada; e um teste
   ponta a ponta que carrega `examples/vendas`, chama `open_in` e executa `vendas` e `receita_por_categoria` com `Engine`, conferindo
   a saída com `format_batches`.
8. Rode `cargo fmt`, `cargo test --workspace` e `cargo build` e corrija tudo. O binário `librebi` deve continuar compilando.
9. Escreva `crates/libreproject/README.md` curto em português com o layout, a API e um exemplo.

## Restrições
Código simples e legível, sem `unsafe`. Não crie nada além do descrito. Não altere o binário `librebi`.
Responda no final com 4 linhas: arquivos criados, quantidade de testes, resultado do cargo test, decisões de formato tomadas.
