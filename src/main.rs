mod handlers;
mod repositories;

use crate::repositories::{
    label::{LabelRepository, LabelRepositoryForDb},
    todo::{TodoRepository, TodoRepositoryForDb},
};
use axum::{
    extract::Extension,
    routing::{delete, get, post},
    Router,
};
use dotenv::dotenv;
use handlers::{
    label::{all_label, create_label, delete_label},
    todo::{all_todo, create_todo, delete_todo, find_todo, update_todo},
};
use hyper::header::CONTENT_TYPE;
use std::{env, net::SocketAddr, sync::Arc};
use tower_http::cors::{Any, CorsLayer, Origin};

#[tokio::main]
async fn main() {
    // logging
    let log_level = env::var("RUST_LOG").unwrap_or("info".to_string());
    env::set_var("RUST_LOG", log_level);
    tracing_subscriber::fmt::init();
    dotenv().ok();

    let database_url = &env::var("DATABASE_URL").expect("undefined [DATABASE_URL]");
    tracing::debug!("start connecting to {}...", database_url);
    let pool = sqlx::PgPool::connect(database_url).await.expect(&format!(
        "failed to connect to database, url: {}",
        database_url
    ));

    let app = create_app(
        TodoRepositoryForDb::new(pool.clone()),
        LabelRepositoryForDb::new(pool.clone()),
    );
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::debug!("listening on {}", addr);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

fn create_app<Todo: TodoRepository, Label: LabelRepository>(
    todo_repository: Todo,
    label_repository: Label,
) -> Router {
    Router::new()
        .route("/", get(root))
        .route("/todos", post(create_todo::<Todo>).get(all_todo::<Todo>))
        .route(
            "/todos/:id",
            get(find_todo::<Todo>)
                .delete(delete_todo::<Todo>)
                .patch(update_todo::<Todo>),
        )
        .route(
            "/labels",
            post(create_label::<Label>).get(all_label::<Label>),
        )
        .route("/labels/:id", delete(delete_label::<Label>))
        .layer(Extension(Arc::new(todo_repository)))
        .layer(Extension(Arc::new(label_repository)))
        .layer(
            CorsLayer::new()
                .allow_origin(Origin::exact("http://localhost:3001".parse().unwrap()))
                .allow_methods(Any)
                .allow_headers(vec![CONTENT_TYPE]),
        )
}

async fn root() -> &'static str {
    "Hello, World!"
}

#[cfg(test)]
mod tests {
    use crate::{
        repositories::label::{test_utils::LabelRepositoryForMemory, Label},
        repositories::todo::{test_utils::TodoRepositoryForMemory, CreateTodo, Todo},
    };

    use super::*;
    use axum::{body::Body, http::Request, response::Response};
    use hyper::{header, Method, StatusCode};
    use tower::ServiceExt;

    fn build_req_with_json(path: &str, method: Method, json_body: String) -> Request<Body> {
        Request::builder()
            .uri(path)
            .method(method)
            .header(header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
            .body(Body::from(json_body))
            .unwrap()
    }

    fn build_req_with_empty(path: &str, method: Method) -> Request<Body> {
        Request::builder()
            .uri(path)
            .method(method)
            .body(Body::empty())
            .unwrap()
    }

    async fn res_to_todo(res: Response) -> Todo {
        let bytes = hyper::body::to_bytes(res.into_body()).await.unwrap();
        let body = String::from_utf8(bytes.to_vec()).unwrap();
        let todo = serde_json::from_str(&body)
            .expect(&format!("failed to convert Todo instance. body: {}", body));
        todo
    }

    async fn res_to_label(res: Response) -> Label {
        let bytes = hyper::body::to_bytes(res.into_body()).await.unwrap();
        let body = String::from_utf8(bytes.to_vec()).unwrap();
        let label = serde_json::from_str(&body)
            .expect(&format!("failed to convert Label instance. body: {}", body));
        label
    }

    #[tokio::test]
    async fn should_return_hello_world() {
        let todo_repository = TodoRepositoryForMemory::new();
        let label_repository = LabelRepositoryForMemory::new();
        let req = Request::builder()
            .uri("/")
            .body(hyper::Body::empty())
            .unwrap();
        let res = create_app(todo_repository, label_repository)
            .oneshot(req)
            .await
            .unwrap();
        let bytes = hyper::body::to_bytes(res.into_body()).await.unwrap();
        let body = String::from_utf8(bytes.to_vec()).unwrap();
        assert_eq!(body, "Hello, World!");
    }

    #[tokio::test]
    async fn should_created_todo() {
        let expected = Todo::new(1, "some todo text".to_string());

        let todo_repository = TodoRepositoryForMemory::new();
        let label_repository = LabelRepositoryForMemory::new();
        let req = build_req_with_json(
            "/todos",
            Method::POST,
            r#"{"text":"some todo text"}"#.to_string(),
        );
        let res = create_app(todo_repository, label_repository)
            .oneshot(req)
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);

        let todo = res_to_todo(res).await;
        assert_eq!(todo, expected);
    }

    #[tokio::test]
    async fn should_find_todo() {
        let expected = Todo::new(1, "some todo text".to_string());

        let todo_repository = TodoRepositoryForMemory::new();
        let label_repository = LabelRepositoryForMemory::new();
        todo_repository
            .create(CreateTodo::new("some todo text".to_string()))
            .await
            .expect("failed to create todo");
        let req = build_req_with_empty("/todos/1", Method::GET);
        let res = create_app(todo_repository, label_repository)
            .oneshot(req)
            .await
            .unwrap();
        let todo = res_to_todo(res).await;
        assert_eq!(todo, expected);
    }

    #[tokio::test]
    async fn should_get_all_todos() {
        let expected = Todo::new(1, "some todo text".to_string());

        let todo_repository = TodoRepositoryForMemory::new();
        let label_repository = LabelRepositoryForMemory::new();
        todo_repository
            .create(CreateTodo::new("some todo text".to_string()))
            .await
            .expect("failed to create todo");
        let req = build_req_with_empty("/todos", Method::GET);
        let res = create_app(todo_repository, label_repository)
            .oneshot(req)
            .await
            .unwrap();
        let bytes = hyper::body::to_bytes(res.into_body()).await.unwrap();
        let body = String::from_utf8(bytes.to_vec()).unwrap();
        let todo: Vec<Todo> = serde_json::from_str(&body)
            .expect(&format!("failed to convert Todo instance. body: {}", body));
        assert_eq!(todo, vec![expected]);
    }

    #[tokio::test]
    async fn should_update_todo() {
        let expected = Todo::new(1, "updated todo text".to_string());

        let todo_repository = TodoRepositoryForMemory::new();
        let label_repository = LabelRepositoryForMemory::new();
        todo_repository
            .create(CreateTodo::new("some todo text".to_string()))
            .await
            .expect("failed to create todo");
        let req = build_req_with_json(
            "/todos/1",
            Method::PATCH,
            r#"{"text":"updated todo text"}"#.to_string(),
        );
        let res = create_app(todo_repository, label_repository)
            .oneshot(req)
            .await
            .unwrap();
        let todo = res_to_todo(res).await;
        assert_eq!(todo, expected);
    }

    #[tokio::test]
    async fn should_delete_todo() {
        let todo_repository = TodoRepositoryForMemory::new();
        let label_repository = LabelRepositoryForMemory::new();
        todo_repository
            .create(CreateTodo::new("some todo text".to_string()))
            .await
            .expect("failed to create todo");
        let req = build_req_with_empty("/todos/1", Method::DELETE);
        let res = create_app(todo_repository, label_repository)
            .oneshot(req)
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn should_create_label() {
        let todo_repository = TodoRepositoryForMemory::new();
        let label_repository = LabelRepositoryForMemory::new();
        let req = build_req_with_json(
            "/labels",
            Method::POST,
            r#"{"name":"some label text"}"#.to_string(),
        );
        let res = create_app(todo_repository, label_repository)
            .oneshot(req)
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);

        let label = res_to_label(res).await;
        assert_eq!(label.name, "some label text");
    }

    #[tokio::test]
    async fn should_get_all_labels() {
        let todo_repository = TodoRepositoryForMemory::new();
        let label_repository = LabelRepositoryForMemory::new();
        label_repository
            .create("some label text".to_string())
            .await
            .expect("failed to create label");
        let req = build_req_with_empty("/labels", Method::GET);
        let res = create_app(todo_repository, label_repository)
            .oneshot(req)
            .await
            .unwrap();
        let bytes = hyper::body::to_bytes(res.into_body()).await.unwrap();
        let body = String::from_utf8(bytes.to_vec()).unwrap();
        let label_vec: Vec<Label> = serde_json::from_str(&body)
            .expect(&format!("failed to convert Label instance. body: {}", body));
        assert_eq!(label_vec.len(), 1);
        assert_eq!(label_vec[0].name, "some label text");
    }

    #[tokio::test]
    async fn should_delete_label() {
        let todo_repository = TodoRepositoryForMemory::new();
        let label_repository = LabelRepositoryForMemory::new();
        label_repository
            .create("some label text".to_string())
            .await
            .expect("failed to create label");
        let req = build_req_with_empty("/labels/1", Method::DELETE);
        let res = create_app(todo_repository, label_repository)
            .oneshot(req)
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::NO_CONTENT);
    }
}
