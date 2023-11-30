use axum::async_trait;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use thiserror::Error;
use validator::Validate;

#[derive(Debug, Error)]
enum RepositoryError {
    #[error("NotFound: {0}")]
    NotFound(i32),
}

#[async_trait]
pub trait TodoRepository: Clone + std::marker::Send + std::marker::Sync + 'static {
    // anyhow::Result<Todo> を返すよう修正
    async fn create(&self, payload: CreateTodo) -> anyhow::Result<Todo>;
    async fn find(&self, id: i32) -> anyhow::Result<Todo>;
    async fn all(&self) -> anyhow::Result<Vec<Todo>>;
    async fn update(&self, id: i32, payload: UpdateTodo) -> anyhow::Result<Todo>;
    async fn delete(&self, id: i32) -> anyhow::Result<()>;
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct Todo {
    id: i32,
    text: String,
    completed: bool,
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
    async fn create(&self, _payload: CreateTodo) -> anyhow::Result<Todo> {
        todo!()
    }

    async fn find(&self, _id: i32) -> anyhow::Result<Todo> {
        todo!()
    }

    async fn all(&self) -> anyhow::Result<Vec<Todo>> {
        todo!()
    }

    async fn update(&self, id: i32, _payload: UpdateTodo) -> anyhow::Result<Todo> {
        todo!()
    }

    async fn delete(&self, _id: i32) -> anyhow::Result<()> {
        todo!()
    }
}

#[cfg(test)]
pub mod test_utils {
    use super::*;
    use anyhow::Context;
    use std::collections::HashMap;
    use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

    impl Todo {
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

    type TodoDatas = HashMap<i32, Todo>;

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
        async fn create(&self, payload: CreateTodo) -> anyhow::Result<Todo> {
            let mut store = self.write_score_ref();
            let id = store.len() as i32 + 1;
            let todo = Todo::new(id, payload.text.clone());
            store.insert(id, todo.clone());
            Ok(todo)
        }

        async fn find(&self, id: i32) -> anyhow::Result<Todo> {
            let store = self.read_score_ref();
            // TODO: Use Box::new
            let todo = store
                .get(&id)
                .map(|todo| todo.clone())
                .ok_or(RepositoryError::NotFound(id))?;
            Ok(todo)
        }

        async fn all(&self) -> anyhow::Result<Vec<Todo>> {
            let store = self.read_score_ref();
            Ok(store.values().cloned().collect())
        }

        async fn update(&self, id: i32, payload: UpdateTodo) -> anyhow::Result<Todo> {
            let mut store = self.write_score_ref();
            let todo = store.get(&id).context(RepositoryError::NotFound(id))?;
            let text = payload.text.unwrap_or(todo.text.clone());
            let completed = payload.completed.unwrap_or(todo.completed);
            let todo = Todo {
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
            let expected = Todo::new(id, text.clone());

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
                Todo {
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
