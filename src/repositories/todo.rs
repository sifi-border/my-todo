use super::{label::Label, RepositoryError};
use anyhow::Ok;
use axum::async_trait;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use validator::Validate;

#[async_trait]
pub trait TodoRepository: Clone + std::marker::Send + std::marker::Sync + 'static {
    // anyhow::Result<Todo> を返すよう修正
    async fn create(&self, payload: CreateTodo) -> anyhow::Result<TodoEntity>;
    async fn find(&self, id: i32) -> anyhow::Result<TodoEntity>;
    async fn all(&self) -> anyhow::Result<Vec<TodoEntity>>;
    async fn update(&self, id: i32, payload: UpdateTodo) -> anyhow::Result<TodoEntity>;
    async fn delete(&self, id: i32) -> anyhow::Result<()>;
}

#[derive(Debug, Clone, PartialEq, Eq, FromRow)]
pub struct TodoWithLabelFromRow {
    id: i32,
    text: String,
    completed: bool,
    label_id: Option<i32>,
    label_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, FromRow)]
struct TodoFromRow {
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
        // 同一id のtodoを畳み込み
        // 同一id の場合、Labelを作成し`labels`にpush
        if let Some(todo) = acc.iter_mut().find(|todo| todo.id == cur.id) {
            if let Some(label_id) = cur.label_id {
                let label = Label {
                    id: label_id,
                    name: cur.label_name.clone().unwrap(),
                };
                todo.labels.push(label);
            }
        // 同一id がない場合、新規todoを作成し`acc`にpush
        } else {
            let mut todo = TodoEntity {
                id: cur.id,
                text: cur.text.clone(),
                completed: cur.completed,
                labels: vec![],
            };
            if let Some(label_id) = cur.label_id {
                let label = Label {
                    id: label_id,
                    name: cur.label_name.clone().unwrap(),
                };
                todo.labels.push(label);
            }
            acc.push(todo);
        }
        acc
    })
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Validate)]
pub struct CreateTodo {
    #[validate(length(min = 1, message = "Can not be empty"))]
    #[validate(length(max = 100, message = "Text is too long"))]
    text: String,
    label_ids: Vec<i32>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Validate)]
pub struct UpdateTodo {
    #[validate(length(min = 1, message = "Can not be empty"))]
    #[validate(length(max = 100, message = "Text is too long"))]
    text: Option<String>,
    completed: Option<bool>,
    label_ids: Option<Vec<i32>>,
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
    async fn create(&self, payload: CreateTodo) -> anyhow::Result<TodoEntity> {
        let tx = self.pool.begin().await?;
        let row = sqlx::query_as::<_, TodoFromRow>(
            r#"
            insert into todos (text, completed)
            values ($1, false)
            returning *;
            "#,
        )
        .bind(payload.text.clone())
        .fetch_one(&self.pool)
        .await?;

        sqlx::query(
            r#"
            insert into todo_labels (todo_id, label_id)
            select $1, id 
            from unnest($2) as t(id)
            "#,
        )
        .bind(row.id)
        .bind(payload.label_ids)
        .execute(&self.pool)
        .await?;

        tx.commit().await?;

        let todo = self.find(row.id).await?;
        Ok(todo)
    }

    async fn find(&self, id: i32) -> anyhow::Result<TodoEntity> {
        let items = sqlx::query_as::<_, TodoWithLabelFromRow>(
            r#"
            select todos.*, labels.id as label_id, labels.name as label_name
            from todos
            left outer join todo_labels tl on todos.id = tl.todo_id
            left outer join labels on labels.id = tl.label_id 
            where todos.id = $1;
            "#,
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => RepositoryError::NotFound(id),
            _ => RepositoryError::Unexpected(e.to_string()),
        })?;
        let todos = fold_entities(items);
        let todo = todos.first().ok_or(RepositoryError::NotFound(id))?;

        Ok(todo.clone())
    }

    async fn all(&self) -> anyhow::Result<Vec<TodoEntity>> {
        let items = sqlx::query_as::<_, TodoWithLabelFromRow>(
            r#"
            select todos.*, labels.id as label_id, labels.name as label_name
            from todos
            left outer join todo_labels tl on todos.id = tl.todo_id
            left outer join labels on labels.id = tl.label_id 
            order by todos.id desc;
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(fold_entities(items))
    }

    async fn update(&self, id: i32, payload: UpdateTodo) -> anyhow::Result<TodoEntity> {
        let tx = self.pool.begin().await?;

        // update todo
        let old_todo = self.find(id).await?;
        sqlx::query(
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

        // update labels
        if let Some(label_ids) = payload.label_ids {
            sqlx::query(
                r#"
                delete from todo_labels where todo_id=$1
                "#,
            )
            .bind(id)
            .execute(&self.pool)
            .await?;

            sqlx::query(
                r#"
                insert into todo_labels (todo_id, label_id)
                select $1, id
                from unnest($2) as t(id)
                "#,
            )
            .bind(id)
            .bind(label_ids)
            .execute(&self.pool)
            .await?;
        };

        tx.commit().await?;
        let todo = self.find(id).await?;

        Ok(todo)
    }

    async fn delete(&self, id: i32) -> anyhow::Result<()> {
        let tx = self.pool.begin().await?;
        // delete todo's label
        sqlx::query(
            r#"
            DELETE FROM todo_labels WHERE todo_id = $1
            "#,
        )
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => RepositoryError::NotFound(id),
            _ => RepositoryError::Unexpected(e.to_string()),
        })?;
        // delete todo
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

        tx.commit().await?;

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn fold_entities_test() {
        let label_1 = Label {
            id: 1,
            name: "label_1".to_string(),
        };
        let label_2 = Label {
            id: 2,
            name: "label_2".to_string(),
        };

        let row = vec![
            TodoWithLabelFromRow {
                id: 1,
                text: "todo_1".to_string(),
                completed: false,
                label_id: Some(label_1.id),
                label_name: Some(label_1.name.clone()),
            },
            TodoWithLabelFromRow {
                id: 1,
                text: "todo_1".to_string(),
                completed: false,
                label_id: Some(label_2.id),
                label_name: Some(label_2.name.clone()),
            },
            TodoWithLabelFromRow {
                id: 2,
                text: "todo_2".to_string(),
                completed: false,
                label_id: Some(label_1.id),
                label_name: Some(label_1.name.clone()),
            },
        ];
        let res = fold_entities(row);
        assert_eq!(
            res,
            vec![
                TodoEntity {
                    id: 1,
                    text: "todo_1".to_string(),
                    completed: false,
                    labels: vec![label_1.clone(), label_2.clone()],
                },
                TodoEntity {
                    id: 2,
                    text: "todo_2".to_string(),
                    completed: false,
                    labels: vec![label_1.clone()],
                },
            ]
        );
    }

    #[cfg(feature = "database-test")]
    #[tokio::test]
    async fn crud_scenario() {
        use dotenv::dotenv;
        use std::vec;

        dotenv().ok();
        let database_url = std::env::var("DATABASE_URL").expect("undefined [DATABASE_URL]");
        let pool = PgPool::connect(&database_url)
            .await
            .expect(&format!("failed to connect to db, url: [{}]", database_url));

        let label_name = "test label".to_string();
        let optional_label = sqlx::query_as::<_, Label>(
            r#"
            select * from labels where name = $1
            "#,
        )
        .bind(label_name.clone())
        .fetch_optional(&pool)
        .await
        .expect("Failed to prepare label data.");
        let label_1 = if let Some(label) = optional_label {
            label
        } else {
            let label = sqlx::query_as::<_, Label>(
                r#"
                insert into labels ( name )
                values ( $1 )
                returning *
                "#,
            )
            .bind(label_name)
            .fetch_one(&pool)
            .await
            .expect("Failed to insert label data.");
            label
        };

        let repository = TodoRepositoryForDb::new(pool.clone());
        let todo_text = "[crud_scenario] todo text".to_string();

        // create
        let created_todo = repository
            .create(CreateTodo::new(todo_text.clone(), vec![label_1.id]))
            .await
            .expect("[create] failed to create todo");
        assert_eq!(created_todo.text, todo_text);
        assert!(!created_todo.completed);
        assert_eq!(*created_todo.labels.first().unwrap(), label_1);

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
                    label_ids: Some(vec![]),
                },
            )
            .await
            .expect("[update] failed to update todo");
        assert_eq!(
            updated_todo,
            TodoEntity {
                id: created_todo.id,
                text: updated_text.clone(),
                completed: true,
                labels: vec![],
            }
        );

        // delete
        let _result = repository
            .delete(created_todo.id)
            .await
            .expect("failed to delete todo");
        let res_after_delete = repository.find(created_todo.id).await;
        assert!(res_after_delete.is_err());

        let rows = sqlx::query(
            r#"
            select * from todo_labels where todo_id=$1
            "#,
        )
        .bind(created_todo.id)
        .fetch_all(&pool)
        .await
        .expect("failed to fetch todo_labels");
        assert!(rows.is_empty());
    }
}

#[cfg(test)]
pub mod test_utils {
    use super::*;
    use anyhow::Context;
    use std::collections::HashMap;
    use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

    impl TodoEntity {
        pub fn new(id: i32, text: String, labels: Vec<Label>) -> Self {
            Self {
                id,
                text,
                completed: false,
                labels,
            }
        }
    }

    impl CreateTodo {
        pub fn new(text: String, label_ids: Vec<i32>) -> Self {
            Self { text, label_ids }
        }
    }

    type TodoDatas = HashMap<i32, TodoEntity>;

    #[derive(Debug, Clone)]
    pub struct TodoRepositoryForMemory {
        store: Arc<RwLock<TodoDatas>>,
        labels: Vec<Label>,
    }

    impl TodoRepositoryForMemory {
        pub fn new(labels: Vec<Label>) -> Self {
            Self {
                store: Arc::default(),
                labels,
            }
        }

        fn write_score_ref(&self) -> RwLockWriteGuard<TodoDatas> {
            self.store.write().unwrap()
        }

        fn read_score_ref(&self) -> RwLockReadGuard<TodoDatas> {
            self.store.read().unwrap()
        }

        fn resolve_labels(&self, label_ids: Vec<i32>) -> Vec<Label> {
            label_ids
                .iter()
                .filter_map(|id| {
                    self.labels
                        .iter()
                        .find(|label| label.id == *id)
                        .map(|label| label.clone())
                })
                .collect()
        }
    }

    #[async_trait]
    impl TodoRepository for TodoRepositoryForMemory {
        async fn create(&self, payload: CreateTodo) -> anyhow::Result<TodoEntity> {
            let mut store = self.write_score_ref();
            let id = store.len() as i32 + 1;
            let labels = self.resolve_labels(payload.label_ids);
            let todo = TodoEntity::new(id, payload.text.clone(), labels);
            store.insert(id, todo.clone());
            Ok(todo)
        }

        async fn find(&self, id: i32) -> anyhow::Result<TodoEntity> {
            let store = self.read_score_ref();
            // TODO: Use Box::new
            let todo = store
                .get(&id)
                .map(|todo| todo.clone())
                .ok_or(RepositoryError::NotFound(id))?;
            Ok(todo)
        }

        async fn all(&self) -> anyhow::Result<Vec<TodoEntity>> {
            let store = self.read_score_ref();
            Ok(store.values().cloned().collect())
        }

        async fn update(&self, id: i32, payload: UpdateTodo) -> anyhow::Result<TodoEntity> {
            let mut store = self.write_score_ref();
            let todo = store.get(&id).context(RepositoryError::NotFound(id))?;
            let text = payload.text.unwrap_or(todo.text.clone());
            let completed = payload.completed.unwrap_or(todo.completed);
            let labels = match payload.label_ids {
                Some(v) => self.resolve_labels(v),
                None => todo.labels.clone(),
            };
            let todo = TodoEntity {
                id,
                text,
                completed,
                labels,
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
            let expected = TodoEntity::new(id, text.clone(), vec![]);

            // create
            let label_ids = vec![];
            let labels = vec![];
            let repository = TodoRepositoryForMemory::new(labels);
            let todo = repository
                .create(CreateTodo::new(text, label_ids))
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
                        label_ids: Some(vec![]),
                    },
                )
                .await
                .expect("failed to update");
            assert_eq!(
                todo,
                TodoEntity {
                    id,
                    text: updated_text.clone(),
                    completed: true,
                    labels: vec![],
                }
            );

            // delete
            let result = repository.delete(id).await;
            assert!(result.is_ok());
        }
    }
}
