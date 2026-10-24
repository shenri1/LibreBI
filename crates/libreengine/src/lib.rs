//! **libreengine** — motor local de execução do LibreBI.
//!
//! O crate registra fontes de dados (CSV e Parquet) em uma sessão do DataFusion,
//! compila consultas do [`librequery`] para SQL e executa o resultado,
//! devolvendo linhas em formato Arrow ([`RecordBatch`]).
//!
//! Também oferece pré-visualização limitada, pré-visualização "até o passo N"
//! (resultado parcial de uma consulta) e a formatação do resultado como tabela
//! de texto.
//!
//! ```
//! # async fn exemplo() -> Result<(), libreengine::EngineError> {
//! use librequery::{Query, Step};
//!
//! let engine = libreengine::Engine::new();
//! engine
//!     .register_csv("vendas", std::path::Path::new("vendas.csv"))
//!     .await?;
//!
//! let query = Query {
//!     id: "vendas_positivas".to_string(),
//!     source: "vendas".to_string(),
//!     steps: vec![Step::Filter("qtd > 0".to_string())],
//! };
//!
//! let batches = engine.preview(&query, 10).await?;
//! println!("{}", libreengine::format_batches(&batches));
//! # Ok(())
//! # }
//! ```

mod engine;
mod format;

pub use engine::{Engine, EngineError};
pub use format::format_batches;
