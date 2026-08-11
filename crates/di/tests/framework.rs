use std::sync::Arc;

use axum::body::Body;
use axum::extract::FromRef;
use axum::http::{Request, StatusCode};
use axum::routing::get;
use axum::Router;
use di::{injectable, module, Container, ContainerBuilder, Injected, Module};
use tower::ServiceExt;

// ─── container primitives ──────────────────────────────────

#[test]
fn insert_and_get_concrete() {
    let mut builder = ContainerBuilder::new();
    builder.insert(42u32);
    let container = builder.build();
    assert_eq!(*container.get::<u32>().unwrap(), 42);
}

#[test]
fn get_missing_returns_none() {
    let container = ContainerBuilder::new().build();
    assert!(container.get::<u32>().is_none());
}

trait Greeter: Send + Sync {
    fn greet(&self) -> String;
}

// Zero-field #[injectable] struct: a leaf provider with no dependencies of its own — every
// field (vacuously none) is "injected", so it still gets an `Injectable` impl.
#[injectable]
struct EnglishGreeter {}
impl Greeter for EnglishGreeter {
    fn greet(&self) -> String {
        "hello".into()
    }
}

#[test]
fn bind_and_get_trait_object() {
    let mut builder = ContainerBuilder::new();
    builder
        .bind::<dyn Greeter>()
        .to_arc(Arc::new(EnglishGreeter {}) as Arc<dyn Greeter>);
    let container = builder.build();
    let greeter = container.get::<dyn Greeter>().expect("bound");
    assert_eq!(greeter.greet(), "hello");
}

// ─── #[injectable] ──────────────────────────────────────────

#[injectable]
#[derive(Debug)]
struct Repo {
    #[inject]
    pool: Arc<u32>,
}

#[injectable]
struct Service {
    #[inject]
    repo: Arc<Repo>,
    #[inject]
    greeter: Arc<dyn Greeter>,
}

#[test]
fn injectable_construct_resolves_dependencies() {
    let mut builder = ContainerBuilder::new();
    builder.insert(7u32);
    builder
        .bind::<dyn Greeter>()
        .to_arc(Arc::new(EnglishGreeter {}) as Arc<dyn Greeter>);

    let repo = <Repo as di::Injectable>::construct(&builder).expect("repo constructs");
    assert_eq!(*repo.pool, 7);
    builder.insert_arc::<Repo>(repo);

    let service = <Service as di::Injectable>::construct(&builder).expect("service constructs");
    assert_eq!(*service.repo.pool, 7);
    assert_eq!(service.greeter.greet(), "hello");
}

#[test]
fn injectable_construct_missing_dependency_errors() {
    let builder = ContainerBuilder::new();
    let err = <Repo as di::Injectable>::construct(&builder).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("Repo"));
    assert!(msg.contains("pool"));
}

// A struct with a plain (non-#[inject]) field still gets a generated constructor, but no
// `Injectable` impl — it must be built by hand and inserted at the composition root rather than
// listed bare inside `#[module(providers = [...])]`.
#[injectable]
struct PlainFieldStruct {
    max_bytes: usize,
}

#[test]
fn plain_fields_still_get_new() {
    let s = PlainFieldStruct::new(5);
    assert_eq!(s.max_bytes, 5);
}

// ─── #[module] ──────────────────────────────────────────────

// Deliberately never binds `dyn Greeter`, so constructing `Service` inside it must fail fast.
#[module(providers = [Repo, Service])]
struct IncompleteModule;

#[module(providers = [EnglishGreeter as dyn Greeter, Repo, Service])]
struct AppModule;

#[test]
fn module_registers_providers_in_order() {
    let mut builder = ContainerBuilder::new();
    builder.insert(99u32);
    AppModule::register(&mut builder).expect("module registers");
    let container = builder.build();

    let service = container.get::<Service>().expect("service registered");
    assert_eq!(*service.repo.pool, 99);
    assert_eq!(service.greeter.greet(), "hello");

    let greeter = container.get::<dyn Greeter>().expect("trait binding registered");
    assert_eq!(greeter.greet(), "hello");
}

#[test]
fn module_missing_provider_fails_fast() {
    let mut builder = ContainerBuilder::new();
    builder.insert(1u32);
    let result = IncompleteModule::register(&mut builder);
    assert!(result.is_err());
}

#[module(imports = [AppModule])]
struct ParentModuleA;

#[module(imports = [AppModule])]
struct ParentModuleB;

#[test]
fn diamond_import_constructs_shared_module_once() {
    let mut builder = ContainerBuilder::new();
    builder.insert(5u32);
    ParentModuleA::register(&mut builder).unwrap();
    ParentModuleB::register(&mut builder).unwrap();
    let container = builder.build();
    assert!(container.get::<Service>().is_some());
}

// ─── Injected<T> extractor ──────────────────────────────────

#[derive(Clone)]
struct AppState {
    container: Container,
}

impl FromRef<AppState> for Container {
    fn from_ref(state: &AppState) -> Self {
        state.container.clone()
    }
}

async fn handler(Injected(service): Injected<Service>) -> String {
    format!("{}:{}", service.greeter.greet(), service.repo.pool)
}

struct NotRegistered;

async fn handler_missing(Injected(_missing): Injected<NotRegistered>) -> &'static str {
    "unreachable"
}

#[tokio::test]
async fn injected_extracts_provider_from_state() {
    let mut builder = ContainerBuilder::new();
    builder.insert(3u32);
    AppModule::register(&mut builder).unwrap();
    let container = builder.build();
    let state = AppState { container };

    let app: Router = Router::new().route("/", get(handler)).with_state(state);
    let response = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    assert_eq!(&body[..], b"hello:3");
}

#[tokio::test]
async fn injected_missing_provider_rejects_with_500() {
    let state = AppState {
        container: ContainerBuilder::new().build(),
    };
    let app: Router = Router::new().route("/", get(handler_missing)).with_state(state);
    let response = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}
