use super::{label::Label, RepositoryError};
use anyhow::Ok;
use axum::async_trait;
use serde::{Deserialize, Serialize};
use sqlx::{prelude::FromRow, PgPool};
use validator::Validate;

#[async_trait]
pub trait TodoRepository: Clone + std::marker::Send + std::marker::Sync + 'static {
    // anyhow::Result<Todo> を返すよう修正
    async fn create(&self, payload: CreateTodo) -> anyhow::Result<TodoWithLabelFromRow>;
    async fn find(&self, id: i32) -> anyhow::Result<TodoWithLabelFromRow>;
    async fn all(&self) -> anyhow::Result<Vec<TodoWithLabelFromRow>>;
    async fn update(&self, id: i32, payload: UpdateTodo) -> anyhow::Result<TodoWithLabelFromRow>;
    async fn delete(&self, id: i32) -> anyhow::Result<()>;
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, FromRow)]
pub struct TodoWithLabelFromRow {
    id: i32,
    text: String,
    completed: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct TodoEntity {
    pub id: i32,
    pub text: String,
    pub completed: bool,
    pub labels: Vec<Label>,
}

fn fold_entities(rows: Vec<TodoWithLabelFromRow>) -> Vec<TodoEntity> {
    rows.iter().fold(vec![], |mut acc: Vec<TodoEntity>, cur| {
        // todo 同一id のtodoを畳み込み
        // todo 同一id の場合、Labelを作成し`labels`にpush
        acc.push(TodoEntity {
            id: cur.id,
            text: cur.text.clone(),
            completed: cur.completed,
            labels: vec![],
        });
        acc
    })
}

fn fold_entity(row: TodoWithLabelFromRow) -> TodoEntity {
    let todo_entities = fold_entities(vec![row]);
    let todo = todo_entities.first().expect("expected 1 todo");

    todo.clone()
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Validate)]
pub struct CreateTodo {
    #[validate(length(min = 1, message = "Can not be empty"))]
    #[validate(length(max = 100, message = "Text is too long"))]
    text: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Validate)]
pub struct UpdateTodo {
    #[validate(length(min = 1, message = "Can not be empty"))]
    #[validate(length(max = 100, message = "Text is too long"))]
    text: Option<String>,
    completed: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct TodoRepositoryForDb {
    pub pool: PgPool,
}

impl TodoRepositoryForDb {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl TodoRepository for TodoRepositoryForDb {
    async fn create(&self, payload: CreateTodo) -> anyhow::Result<TodoWithLabelFromRow> {
        let todo = sqlx::query_as::<_, TodoWithLabelFromRow>(
            r#"
            INSERT INTO todos (text, completed)
            VALUES ($1, false)
            RETURNING id, text, completed
            "#,
        )
        .bind(payload.text.clone())
        .fetch_one(&self.pool)
        .await?;

        Ok(todo)
    }

    async fn find(&self, id: i32) -> anyhow::Result<TodoWithLabelFromRow> {
        let todo = sqlx::query_as::<_, TodoWithLabelFromRow>(
            r#"
            SELECT * FROM todos WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => RepositoryError::NotFound(id),
            _ => RepositoryError::Unexpected(e.to_string()),
        })?;

        Ok(todo)
    }

    async fn all(&self) -> anyhow::Result<Vec<TodoWithLabelFromRow>> {
        let todo_vec = sqlx::query_as::<_, TodoWithLabelFromRow>(
            r#"
            SELECT * FROM todos ORDER BY id DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(todo_vec)
    }

    async fn update(&self, id: i32, payload: UpdateTodo) -> anyhow::Result<TodoWithLabelFromRow> {
        let old_todo = self.find(id).await?;
        let todo = sqlx::query_as::<_, TodoWithLabelFromRow>(
            r#"
            UPDATE todos SET text = $1, completed = $2 WHERE id = $3
            RETURNING *
            "#,
        )
        .bind(payload.text.unwrap_or(old_todo.text))
        .bind(payload.completed.unwrap_or(old_todo.completed))
        .bind(id)
        .fetch_one(&self.pool)
        .await?;

        Ok(todo)
    }

    async fn delete(&self, id: i32) -> anyhow::Result<()> {
        sqlx::query(
            r#"
            DELETE FROM todos WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => RepositoryError::NotFound(id),
            _ => RepositoryError::Unexpected(e.to_string()),
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

    #[tokio::test]
    async fn crud_scenario() {
        dotenv().ok();
        let database_url = std::env::var("DATABASE_URL").expect("undefined [DATABASE_URL]");
        let pool = PgPool::connect(&database_url)
            .await
            .expect(&format!("failed to connect to db, url: [{}]", database_url));
        let repository = TodoRepositoryForDb::new(pool.clone());
        let todo_text = "[crud_scenario] todo text".to_string();

        // create
        let created_todo = repository
            .create(CreateTodo::new(todo_text.clone()))
            .await
            .expect("[create] failed to create todo");
        assert_eq!(created_todo.text, todo_text);
        assert!(!created_todo.completed);

        // find
        let found_todo = repository
            .find(created_todo.id)
            .await
            .expect("[find] failed to find todo");
        assert_eq!(found_todo, created_todo);

        // all
        let todos = repository
            .all()
            .await
            .expect("[all] failed to get all todos");
        assert_eq!(*todos.first().unwrap(), created_todo.clone());

        // update
        let updated_text = "[crud_scenario] updated todo text".to_string();
        let updated_todo = repository
            .update(
                created_todo.id,
                UpdateTodo {
                    text: Some(updated_text.clone()),
                    completed: Some(true),
                },
            )
            .await
            .expect("[update] failed to update todo");
        assert_eq!(
            updated_todo,
            TodoWithLabelFromRow {
                id: created_todo.id,
                text: updated_text.clone(),
                completed: true,
            }
        );

        // delete
        let _result = repository
            .delete(created_todo.id)
            .await
            .expect("failed to delete todo");
        let res_after_delete = repository.find(created_todo.id).await;
        assert!(res_after_delete.is_err());
    }
}

#[cfg(test)]
pub mod test_utils {
    use super::*;
    use anyhow::Context;
    use std::collections::HashMap;
    use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

    impl TodoWithLabelFromRow {
        pub fn new(id: i32, text: String) -> Self {
            Self {
                id,
                text,
                completed: false,
            }
        }
    }

    impl CreateTodo {
        pub fn new(text: String) -> Self {
            Self { text }
        }
    }

    type TodoDatas = HashMap<i32, TodoWithLabelFromRow>;

    #[derive(Debug, Clone)]
    pub struct TodoRepositoryForMemory {
        store: Arc<RwLock<TodoDatas>>,
    }

    impl TodoRepositoryForMemory {
        pub fn new() -> Self {
            Self {
                store: Arc::default(),
            }
        }

        fn write_score_ref(&self) -> RwLockWriteGuard<TodoDatas> {
            self.store.write().unwrap()
        }

        fn read_score_ref(&self) -> RwLockReadGuard<TodoDatas> {
            self.store.read().unwrap()
        }
    }

    #[async_trait]
    impl TodoRepository for TodoRepositoryForMemory {
        async fn create(&self, payload: CreateTodo) -> anyhow::Result<TodoWithLabelFromRow> {
            let mut store = self.write_score_ref();
            let id = store.len() as i32 + 1;
            let todo = TodoWithLabelFromRow::new(id, payload.text.clone());
            store.insert(id, todo.clone());
            Ok(todo)
        }

        async fn find(&self, id: i32) -> anyhow::Result<TodoWithLabelFromRow> {
            let store = self.read_score_ref();
            // TODO: Use Box::new
            let todo = store
                .get(&id)
                .map(|todo| todo.clone())
                .ok_or(RepositoryError::NotFound(id))?;
            Ok(todo)
        }

        async fn all(&self) -> anyhow::Result<Vec<TodoWithLabelFromRow>> {
            let store = self.read_score_ref();
            Ok(store.values().cloned().collect())
        }

        async fn update(
            &self,
            id: i32,
            payload: UpdateTodo,
        ) -> anyhow::Result<TodoWithLabelFromRow> {
            let mut store = self.write_score_ref();
            let todo = store.get(&id).context(RepositoryError::NotFound(id))?;
            let text = payload.text.unwrap_or(todo.text.clone());
            let completed = payload.completed.unwrap_or(todo.completed);
            let todo = TodoWithLabelFromRow {
                id,
                text,
                completed,
            };
            store.insert(id, todo.clone());
            Ok(todo)
        }

        async fn delete(&self, id: i32) -> anyhow::Result<()> {
            let mut store = self.write_score_ref();
            store.remove(&id).ok_or(RepositoryError::NotFound(id))?;
            Ok(())
        }
    }

    mod test {
        use super::*;

        #[tokio::test]
        async fn todo_crud_scenario() {
            let text = "todo text".to_string();
            let id = 1;
            let expected = TodoWithLabelFromRow::new(id, text.clone());

            // create
            let repository = TodoRepositoryForMemory::new();
            let todo = repository
                .create(CreateTodo { text: text.clone() })
                .await
                .expect("failed to create todo");
            assert_eq!(todo, expected);

            // find
            let todo = repository.find(todo.id).await.unwrap();
            assert_eq!(todo, expected);

            // all
            let todos = repository.all().await.expect("failed to get all todos");
            assert_eq!(todos, vec![expected.clone()]);

            // update
            let updated_text = "updated todo text".to_string();
            let todo = repository
                .update(
                    id,
                    UpdateTodo {
                        text: Some(updated_text.clone()),
                        completed: Some(true),
                    },
                )
                .await
                .expect("failed to update");
            assert_eq!(
                todo,
                TodoWithLabelFromRow {
                    id,
                    text: updated_text.clone(),
                    completed: true,
                }
            );

            // delete
            let result = repository.delete(id).await;
            assert!(result.is_ok());
        }
    }
}
