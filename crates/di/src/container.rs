use std::any::{Any, TypeId};
use std::collections::{HashMap, HashSet};
use std::marker::PhantomData;
use std::sync::Arc;

type BoxedProvider = (Box<dyn Any + Send + Sync>, &'static str);

/// Accumulates providers before the container is frozen. Passed by `&mut` to
/// `Injectable::construct` and `Module::register` during bootstrap.
pub struct ContainerBuilder {
    providers: HashMap<TypeId, BoxedProvider>,
    registered_modules: HashSet<TypeId>,
}

impl Default for ContainerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ContainerBuilder {
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
            registered_modules: HashSet::new(),
        }
    }

    /// Registers a pre-built value under its own concrete type (config, `PgPool`, `Mailer`…).
    pub fn insert<T: Send + Sync + 'static>(&mut self, value: T) -> &mut Self {
        self.insert_arc(Arc::new(value))
    }

    /// Registers an already-`Arc`'d value, keyed by `T` — pass `T = dyn Trait` to bind an
    /// interface, or a concrete type to register it directly.
    pub fn insert_arc<T: ?Sized + Send + Sync + 'static>(&mut self, value: Arc<T>) -> &mut Self {
        self.providers.insert(
            TypeId::of::<Arc<T>>(),
            (Box::new(value), std::any::type_name::<T>()),
        );
        self
    }

    /// Interface-binding sugar: `builder.bind::<dyn UserRepository>().to_arc(arc_impl)`.
    pub fn bind<Trait: ?Sized + Send + Sync + 'static>(&mut self) -> Binder<'_, Trait> {
        Binder {
            builder: self,
            _t: PhantomData,
        }
    }

    pub fn get<T: ?Sized + 'static>(&self) -> Option<Arc<T>> {
        self.providers
            .get(&TypeId::of::<Arc<T>>())
            .and_then(|(b, _)| b.downcast_ref::<Arc<T>>())
            .cloned()
    }

    /// Returns `true` the first time a given module marker type is registered, `false` on any
    /// subsequent call — lets `#[module(imports = [...])]` import the same shared module from
    /// multiple places without constructing its providers more than once.
    pub fn mark_registered<M: 'static>(&mut self) -> bool {
        self.registered_modules.insert(TypeId::of::<M>())
    }

    /// Freezes the builder into a cheaply-`Clone`-able `Container`.
    pub fn build(self) -> Container {
        Container(Arc::new(self.providers))
    }
}

pub struct Binder<'a, Trait: ?Sized> {
    builder: &'a mut ContainerBuilder,
    _t: PhantomData<Trait>,
}

impl<'a, Trait: ?Sized + Send + Sync + 'static> Binder<'a, Trait> {
    pub fn to_arc(self, value: Arc<Trait>) {
        self.builder.insert_arc::<Trait>(value);
    }
}

/// The frozen, `Clone`-cheap (`Arc`-backed) provider registry. Reachable from Axum handlers via
/// `Injected<T>`, and from `AppState` via `FromRef`.
#[derive(Clone)]
pub struct Container(Arc<HashMap<TypeId, BoxedProvider>>);

impl Container {
    pub fn get<T: ?Sized + 'static>(&self) -> Option<Arc<T>> {
        self.0
            .get(&TypeId::of::<Arc<T>>())
            .and_then(|(b, _)| b.downcast_ref::<Arc<T>>())
            .cloned()
    }
}
