//! Configuration module for CleanMac.

pub mod loader;
pub mod schema;

pub use loader::{
    create_default_config, default_config_path, expand_tilde, load_config, validate_config,
};
pub use schema::Config;
