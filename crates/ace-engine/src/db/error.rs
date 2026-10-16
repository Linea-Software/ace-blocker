use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum DBError {
    #[error("[DB ERROR] Error while fetching data from table {table}: {message} ")]
    DataFetchError { table: String, message: String },
    #[error("[DB ERROR] Couldn't convert data: {message}")]
    DataConversionError { message: String },
    #[error("[DB ERROR] Error during insertion into table {table}: {message}")]
    InsertionError { table: String, message: String },
    #[error("[DB ERROR] Error during update of table {table}: {message}")]
    UpdateError { table: String, message: String },

    #[error("[DB ERROR] Error during deletion from table {table}: {message}")]
    DeletionError { table: String, message: String },

    #[error("[DB ERROR] Connection to the database failed: {message}")]
    DatabaseConnectionError { message: String },

    #[error("[DB ERROR] Database migrations failed: {message}")]
    MigrationError { message: String },
}
