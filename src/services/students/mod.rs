use sqlx::PgPool;

pub(super) mod admission;
pub(super) mod crud;
pub(super) mod export;
pub(super) mod import;
pub(super) mod promote;

pub struct StudentsService {
    pub(super) pool: PgPool,
}

impl StudentsService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}
