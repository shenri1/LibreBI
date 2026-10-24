//! **librequery** — o tratador de dados do LibreBI.
//!
//! Uma consulta é descrita como uma fonte mais uma lista de passos
//! ([`Query`]). A biblioteca compila essa lista em um único texto SQL composto
//! por uma cadeia de CTEs ([`compile`]). Nesta versão não há execução de SQL nem
//! leitura de dados.
//!
//! ```
//! use librequery::{compile, DataType, Query, Step};
//!
//! let query = Query {
//!     id: "vendas".to_string(),
//!     source: "src_arquivo_vendas".to_string(),
//!     steps: vec![
//!         Step::Filter("qtd > 0".to_string()),
//!         Step::ChangeType { column: "data".to_string(), to: DataType::Date },
//!     ],
//! };
//!
//! let sql = compile(&query).unwrap();
//! assert!(sql.contains("WHERE qtd > 0"));
//! ```

pub mod compile;
pub mod model;
pub mod sql;

pub use compile::{CompileError, compile};
pub use model::{AggFunc, Aggregation, DataType, Direction, JoinKind, Query, Step};
