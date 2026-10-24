//! **libreproject** — o projeto do LibreBI como pasta de texto.
//!
//! O crate lê e grava um projeto em uma pasta versionável em git, valida as
//! referências entre fontes e consultas e registra tudo no [`libreengine`] para
//! execução. Não há interface gráfica aqui.
//!
//! - [`Project::load`] lê `project.yaml`, `sources/*.yaml` e `queries/*.yaml`;
//! - [`Project::save`] grava de forma canônica e remove arquivos obsoletos;
//! - [`Project::validate`] confere ids, referências e ciclos;
//! - [`Project::query_order`] devolve as consultas em ordem de dependência;
//! - [`Project::open_in`] registra fontes e consultas no motor.
//!
//! O layout é uma pasta versionável em git:
//!
//! ```text
//! meu-projeto/
//! ├─ project.yaml
//! ├─ sources/<id>.yaml
//! └─ queries/<id>.yaml
//! ```
//!
//! A gravação é canônica: campos em ordem fixa, listas ordenadas por id,
//! indentação fixa e `\n` como terminador de linha. Dois salvamentos seguidos
//! produzem arquivos idênticos.
//!
//! ```
//! use libreproject::{Project, Query, SourceDef, SourceKind, Step};
//!
//! let project = Project {
//!     name: "Vendas".to_string(),
//!     format_version: libreproject::FORMAT_VERSION,
//!     sources: vec![SourceDef {
//!         id: "arquivo_vendas".to_string(),
//!         kind: SourceKind::Csv,
//!         path: "data/vendas.csv".to_string(),
//!     }],
//!     queries: vec![Query {
//!         id: "vendas".to_string(),
//!         source: "arquivo_vendas".to_string(),
//!         steps: vec![Step::Filter("qtd > 0".to_string())],
//!     }],
//! };
//!
//! project.validate().unwrap();
//! ```

mod error;
mod format;
mod project;

pub use crate::error::ProjectError;
pub use crate::project::{FORMAT_VERSION, Project, SourceDef, SourceKind};
pub use librequery::{Query, Step};
