use std::{
    any::{Any, TypeId},
    rc::Rc,
};

use super::ReactiveSystem;

impl ReactiveSystem {
    /// Provide a context value in the current scope.
    ///
    /// This makes the value available to all child scopes via `use_context`.
    /// Uses Cow (clone-on-write) for efficient context inheritance.
    ///
    /// # Type Safety
    /// Context values are stored by their `TypeId`, ensuring type-safe retrieval.
    ///
    /// # Example
    /// ```rust
    /// # use samara_signals::{provide_context, scope};
    /// struct Theme(String);
    ///
    /// scope(|| {
    ///     provide_context(Theme(String::from("dark")));
    /// });
    /// ```
    pub fn provide_context<T: 'static>(&mut self, value: T) {
        let current = self.current_scope.get();
        self.contexts
            .entry(current)
            .unwrap()
            .or_default()
            .insert(TypeId::of::<T>(), Rc::new(value) as Rc<dyn Any>);
    }

    /// Use a context value from the current or any parent scope.
    ///
    /// This walks up the parent chain to find the nearest context of the given type.
    /// Returns `None` if no context of the requested type is found.
    ///
    /// # Type Safety
    /// The generic type parameter `T` determines which context to retrieve.
    ///
    /// # Example
    /// ```rust
    /// # use samara_signals::{provide_context, scope, use_context};
    /// #[derive(Clone)]
    /// struct Theme(String);
    ///
    /// scope(|| {
    ///     provide_context(Theme(String::from("dark")));
    ///
    ///     scope(|| {
    ///         let theme = use_context::<Theme>().unwrap();
    ///         assert_eq!(theme.0, "dark");
    ///     });
    /// });
    /// ```
    pub fn use_context<T: 'static + Clone>(&mut self) -> Option<T> {
        let type_id = TypeId::of::<T>();

        // Walk parent chain to find the context
        let mut current = self.current_scope.get();
        loop {
            if let Some(value) = self
                .contexts
                .get(current)
                .and_then(|contexts| contexts.get(&type_id))
            {
                return value.downcast_ref::<T>().cloned();
            }
            match self.nodes[current].parent {
                Some(parent) => current = parent,
                None => return None,
            }
        }
    }

    /// Check if a context of the given type exists in the current or any parent scope.
    ///
    /// This is useful for conditional logic or providing default values.
    ///
    /// # Example
    /// ```rust
    /// # use samara_signals::{has_context, provide_context, scope};
    /// struct Theme(String);
    ///
    /// scope(|| {
    ///     assert!(!has_context::<Theme>());
    ///
    ///     provide_context(Theme(String::from("dark")));
    ///     assert!(has_context::<Theme>());
    /// });
    /// ```
    pub fn has_context<T: 'static>(&self) -> bool {
        let type_id = TypeId::of::<T>();

        // Walk parent chain to check for context existence
        let mut current = self.current_scope.get();
        loop {
            if let Some(contexts) = self.contexts.get(current)
                && contexts.contains_key(&type_id)
            {
                return true;
            }
            match self.nodes[current].parent {
                Some(parent) => current = parent,
                None => return false,
            }
        }
    }
}
