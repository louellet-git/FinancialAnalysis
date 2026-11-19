//! mapcheck: from SEC data to the standard DCF input variables (fetch, map, check,
//! filing statements, cross-check). See rust/mapcheck/README.md.

pub mod checks;
pub mod cli;
pub mod crosscheck;
pub mod dates;
pub mod facts;
pub mod filing;
pub mod http;
pub mod import;
pub mod mapper;
pub mod registry;
pub mod run;
pub mod sec;
pub mod store;
pub mod supplement;
pub mod summary;
