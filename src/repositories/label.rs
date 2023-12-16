use axum::async_trait;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use validator::Validate;

use super::RepositoryError;

#[async_trait]
pub trait LabelRepository: Clone + std::marker::Send + std::marker::Sync + 'static {
    async fn create(&self, name: String) -> anyhow::Result<Label>;
    async fn all(&self) -> anyhow::Result<Vec<Label>>;
    async fn delete(&self, id: i32) -> anyhow::Result<()>;
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct Label {
    pub id: i32,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Validate)]
pub struct UpdateLabel {
    id: i32,
    #[validate(length(min = 1, message = "Can not be empty"))]
    #[validate(length(max = 20, message = "Name is too long"))]
    name: String,
}

#[derive(Debug, Clone)]
pub struct LabelRepositoryForDb {
    pool: PgPool,
}

impl LabelRepositoryForDb {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl LabelRepository for LabelRepositoryForDb {
    async fn create(&self, name: String) -> anyhow::Result<Label> {
        let optional_label = sqlx::query_as::<_, Label>(
            r#"
            SELECT * FROM LABELS WHERE NAME = $1
            "#,
        )
        .bind(name.clone())
        .fetch_optional(&self.pool)
        .await?;
        if let Some(label) = optional_label {
            return Err(RepositoryError::Duplicate(label.id).into());
        }

        let label = sqlx::query_as::<_, Label>(
            r#"
            INSERT INTO LABELS (NAME) VALUES ($1) RETURNING *
            "#,
        )
        .bind(name.clone())
        .fetch_one(&self.pool)
        .await?;

        Ok(label)
    }
    async fn all(&self) -> anyhow::Result<Vec<Label>> {
        let label_vec = sqlx::query_as::<_, Label>(
            r#"
            SELECT * FROM LABELS ORDER BY ID ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(label_vec)
    }
    async fn delete(&self, id: i32) -> anyhow::Result<()> {
        sqlx::query(
            r#"
            DELETE FROM LABELS WHERE ID = $1
            "#,
        )
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => RepositoryError::NotFound(id),
            _ => RepositoryError::Unexpected(e.to_string()).into(),
        })?;

        Ok(())
    }
}

#[cfg(test)]
#[cfg(feature = "database-test")]
mod test {
    use super::*;
    use dotenv::dotenv;
    use sqlx::PgPool;
    use std::env;

    #[tokio::test]
    async fn crud_scenario() {
        dotenv().ok();
        let database_url = env::var("DATABASE_URL").expect("undefined [DATABASE_URL]");
        let pool = PgPool::connect(&database_url)
            .await
            .expect(&format!("failed to connect to db, url: [{}]", database_url));
        let repository = LabelRepositoryForDb::new(pool);
        let label_text = "test_label".to_string();

        // create
        let label = repository
            .create(label_text.to_string())
            .await
            .expect(&format!("[create] failed to create label"));
        assert_eq!(label.name, label_text);

        // all
        let labels = repository
            .all()
            .await
            .expect("[all] failed to get all labels");
        assert_eq!(labels.len(), 1);
        assert_eq!(labels[0].name, label_text);

        // delete
        repository
            .delete(label.id)
            .await
            .expect("[delete] failed to delete label");
        let labels = repository.all().await.unwrap();
        assert_eq!(labels.len(), 0);
    }
}

#[cfg(test)]
pub mod test_utils {
    use anyhow::Ok;

    use super::*;
    use std::collections::HashMap;
    use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

    impl Label {
        pub fn new(id: i32, name: String) -> Self {
            Self { id, name }
        }
    }

    type LabelData = HashMap<i32, Label>;

    #[derive(Debug, Clone)]
    pub struct LabelRepositoryForMemory {
        data: Arc<RwLock<LabelData>>,
    }

    impl LabelRepositoryForMemory {
        pub fn new() -> Self {
            Self {
                data: Arc::default(),
            }
        }

        fn write_store_ref(&self) -> RwLockWriteGuard<LabelData> {
            self.data.write().unwrap()
        }

        fn read_store_ref(&self) -> RwLockReadGuard<LabelData> {
            self.data.read().unwrap()
        }
    }

    #[async_trait]
    impl LabelRepository for LabelRepositoryForMemory {
        async fn create(&self, name: String) -> anyhow::Result<Label> {
            let mut store = self.write_store_ref();
            let id = store.len() as i32 + 1;
            let label = Label::new(id, name.clone());
            store.insert(id, label.clone());
            Ok(label)
        }

        async fn all(&self) -> anyhow::Result<Vec<Label>> {
            let store = self.read_store_ref();
            let labels = store.values().cloned().collect();
            Ok(labels)
        }

        async fn delete(&self, id: i32) -> anyhow::Result<()> {
            let mut store = self.write_store_ref();
            store.remove(&id);
            Ok(())
        }
    }

    mod test {
        use super::*;

        #[tokio::test]
        async fn crud_scenario() {
            let repository = LabelRepositoryForMemory::new();
            let label_text = "test_label".to_string();

            // create
            let label = repository
                .create(label_text.to_string())
                .await
                .expect(&format!("[create] failed to create label"));
            assert_eq!(label.name, label_text);

            // all
            let labels = repository
                .all()
                .await
                .expect("[all] failed to get all labels");
            assert_eq!(labels.len(), 1);
            assert_eq!(labels[0].name, label_text);

            // delete
            repository
                .delete(label.id)
                .await
                .expect("[delete] failed to delete label");
            let labels = repository.all().await.unwrap();
            assert_eq!(labels.len(), 0);
        }
    }
}
