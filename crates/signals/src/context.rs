use crate::runtime::REACTIVE_SYSTEM;

/// Provide a context value in the current scope.
///
/// This makes the value available to all child scopes via [`use_context`].
/// Context values are stored by their type, ensuring type-safe retrieval.
///
/// # Type Safety
///
/// Each type can have only one context value at a time. If you call
/// `provide_context<T>` multiple times with the same type `T` in the
/// same scope, the most recent value will overwrite the previous one.
///
/// # Context Inheritance
///
/// Child scopes automatically inherit all contexts from their parent scopes.
/// This inheritance is efficient - it uses clone-on-write (Cow) so that
/// child scopes share the parent's context map until the child provides
/// its own context.
///
/// # Example
///
/// ```rust
/// # use samara_signals::{scope, provide_context, use_context};
///
/// scope(|| {
///     provide_context(1);
///
///     // Child scope can access the context
///     scope(|| {
///         let n = use_context::<i32>().unwrap();
///         assert_eq!(n, 1);
///     });
/// });
/// ```
///
/// # Context Shadowing
///
/// A child scope can shadow a parent's context by providing a new value
/// of the same type:
///
/// ```rust
/// # use samara_signals::{scope, provide_context, use_context};
/// #[derive(Clone)]
/// struct Config(i32);
///
/// scope(|| {
///     provide_context(Config(10));
///
///     scope(|| {
///         provide_context(Config(20));  // Shadows parent's Config
///         let config = use_context::<Config>().unwrap();
///         assert_eq!(config.0, 20);
///     });
/// });
/// ```
pub fn provide_context<T: 'static>(value: T) {
    REACTIVE_SYSTEM.with(|ctx| unsafe {
        let ctx = &mut *ctx.get();
        ctx.provide_context(value);
    });
}

/// Use a context value from the current or any parent scope.
///
/// This walks up the parent chain to find the nearest context of the given type.
/// Returns `None` if no context of the requested type is found.
///
/// # Type Parameters
///
/// The generic type parameter `T` determines which context to retrieve.
/// The type must implement `Clone` since the context value is cloned when retrieved.
///
/// # Context Lookup
///
/// The lookup algorithm searches:
/// 1. The current scope
/// 2. The parent scope
/// 3. The grandparent scope
/// 4. ... continuing up the chain
/// 5. Returns `None` if the root is reached without finding a match
///
/// # Example
///
/// ```rust
/// # use samara_signals::{scope, provide_context, use_context};
/// #[derive(Clone, PartialEq, Debug)]
/// enum Theme {
/// 	Light,
/// 	Dark,
/// };
///
/// scope(|| {
///     provide_context(Theme::Dark);
///
///     scope(|| {
///         // Context from parent scope is accessible
///         let theme = use_context::<Theme>().unwrap();
///         assert_eq!(theme, Theme::Dark);
///     });
/// });
///
/// // Context is not accessible outside the providing scope
/// assert!(use_context::<Theme>().is_none());
/// ```
pub fn use_context<T: 'static + Clone>() -> Option<T> {
    REACTIVE_SYSTEM.with(|ctx| unsafe {
        let ctx = &mut *ctx.get();
        ctx.use_context()
    })
}

/// Check if a context of the given type exists in the current or any parent scope.
///
/// This is useful for conditional logic or providing default values.
///
/// # Example
///
/// ```rust
/// # use samara_signals::{scope, provide_context, has_context};
/// enum Theme {
/// 	Dark
/// };
///
/// scope(|| {
///     assert!(!has_context::<Theme>());
///
///     provide_context(Theme::Dark);
///     assert!(has_context::<Theme>());
///
///     scope(|| {
///         // Child scopes see parent's contexts
///         assert!(has_context::<Theme>());
///     });
/// });
/// ```
pub fn has_context<T: 'static>() -> bool {
    REACTIVE_SYSTEM.with(|ctx| unsafe {
        let ctx = &*ctx.get();
        ctx.has_context::<T>()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{scope, signal};

    #[derive(PartialEq, Clone, Debug)]
    enum Theme {
        Light,
        Dark,
    }
    #[test]
    fn test_provide_and_use_context() {
        provide_context(Theme::Light);
        scope(|| {
            provide_context(Theme::Dark);

            let theme = use_context::<Theme>().unwrap();
            assert_eq!(theme, Theme::Dark);
        });
    }

    #[test]
    fn test_context_inheritance() {
        scope(|| {
            provide_context(Theme::Dark);

            scope(|| {
                // Child can access parent's context
                let theme = use_context::<Theme>().unwrap();
                assert_eq!(theme, Theme::Dark);
            });
        });
    }

    #[test]
    fn test_context_not_found() {
        scope(|| {
            let theme = use_context::<Theme>();
            assert!(theme.is_none());
        });
    }

    #[test]
    fn test_has_context() {
        scope(|| {
            assert!(!has_context::<Theme>());

            provide_context(Theme::Dark);
            assert!(has_context::<Theme>());

            scope(|| {
                // Child sees parent's context
                assert!(has_context::<Theme>());
            });
        });
    }

    #[test]
    fn test_context_with_signals() {
        scope(|| {
            provide_context(Theme::Dark);

            scope(move || {
                // Can use both context and signals
                let sig = signal(42);

                let theme = use_context::<Theme>().unwrap();
                assert_eq!(theme, Theme::Dark);

                let value = sig.get();
                assert_eq!(value, 42);
            });
        });
    }
}
