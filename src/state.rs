use sqlx::PgPool;
use std::sync::Arc;

use crate::config::AppConfig;
use crate::services::organization::OrganizationService;
use crate::services::user::UserService;
use crate::services::workos::WorkOsService;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<AppConfig>,
    pub db_pool: PgPool,
    pub workos_service: Arc<WorkOsService>,
    pub user_service: Arc<UserService>,
    pub organization_service: Arc<OrganizationService>,
}

impl AppState {
    pub fn new(config: AppConfig, db_pool: PgPool) -> Self {
        let workos_service = Arc::new(WorkOsService::new(config.workos.clone()));
        let user_service = Arc::new(UserService::new(db_pool.clone()));
        let organization_service = Arc::new(OrganizationService::new(db_pool.clone()));

        Self {
            config: Arc::new(config),
            db_pool,
            workos_service,
            user_service,
            organization_service,
        }
    }
}
