use std::any::{Any, TypeId};
use std::collections::HashMap;

/// A `HashMap`-like key-value store that uses the type of a value
/// as a key.
#[derive(Default)]
pub struct TypeMap(HashMap<TypeId, Box<(dyn Any + Send + Sync)>>);

impl TypeMap {
    /// Creates a new empty TypeMap.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns a reference to the value of the type `T`.
    pub fn get<T>(&self) -> Option<&T>
    where
        T: Any,
    {
        self.0
            .get(&TypeId::of::<T>())
            .and_then(|b| b.downcast_ref::<T>())
    }

    /// Returns a mutable reference to the value of the type
    /// `T`.
    pub fn get_mut<T>(&mut self) -> Option<&mut T>
    where
        T: Any,
    {
        self.0
            .get_mut(&TypeId::of::<T>())
            .and_then(|b| b.downcast_mut::<T>())
    }

    /// Inserts a value into the `TypeMap` using the type `T`
    /// as a key.
    pub fn insert<T>(&mut self, value: T)
    where
        T: Any + Send + Sync,
    {
        self.0.insert(TypeId::of::<T>(), Box::new(value));
    }

    /// Removes and returns the value of the type `T` from the
    /// TypeMap.
    pub fn remove<T>(&mut self) -> Option<T>
    where
        T: Any,
    {
        self.0
            .remove(&TypeId::of::<T>())
            .and_then(|b| Some(*b.downcast::<T>().unwrap()))
    }

    /// Returns `true` if the TypeMap contains an item of the
    /// type `T`.
    pub fn contains_key<T>(&self) -> bool
    where
        T: Any,
    {
        self.0.contains_key(&TypeId::of::<T>())
    }
}
