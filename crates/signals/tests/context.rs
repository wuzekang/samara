use samara_signals::{computed, effect, provide_context, scope, scoped, signal, use_context};

#[derive(Clone, Debug, PartialEq)]
struct Theme(String);

#[derive(Clone, Debug, PartialEq)]
struct Config(i32);

#[test]
fn test_integration_basic_provide_use() {
    scope(|| {
        provide_context(Theme(String::from("dark")));
        let theme = use_context::<Theme>().unwrap();
        assert_eq!(theme, Theme(String::from("dark")));
    });
}

#[test]
fn test_integration_context_inheritance() {
    scope(|| {
        provide_context(Theme(String::from("light")));

        let parent_theme = use_context::<Theme>().unwrap();
        assert_eq!(parent_theme, Theme(String::from("light")));

        scope(|| {
            // Child inherits parent's context
            let child_theme = use_context::<Theme>().unwrap();
            assert_eq!(child_theme, Theme(String::from("light")));
        });
    });
}

#[test]
fn test_integration_context_shadowing() {
    scope(|| {
        provide_context(Config(10));
        assert_eq!(use_context::<Config>().unwrap(), Config(10));

        scope(|| {
            provide_context(Config(20)); // Shadows parent
            assert_eq!(use_context::<Config>().unwrap(), Config(20));
        });

        // Back in parent, original value is restored
        assert_eq!(use_context::<Config>().unwrap(), Config(10));
    });
}

#[test]
fn test_integration_multiple_context_types() {
    scope(|| {
        provide_context(Theme(String::from("dark")));
        provide_context(Config(42));

        let theme = use_context::<Theme>().unwrap();
        let config = use_context::<Config>().unwrap();

        assert_eq!(theme, Theme(String::from("dark")));
        assert_eq!(config, Config(42));
    });
}

#[test]
fn test_integration_context_with_effects() {
    use std::cell::RefCell;
    use std::rc::Rc;

    let effect_runs = Rc::new(RefCell::new(Vec::new()));
    let effect_runs_for_scope = effect_runs.clone();

    scope(move || {
        provide_context(Theme(String::from("dark")));
        let count = signal(0);
        let effect_runs_clone = effect_runs_for_scope.clone();

        effect(move || {
            let theme = use_context::<Theme>().unwrap();
            let n = count.get();
            effect_runs_clone.borrow_mut().push((n, theme.clone()));
        });

        count.set(1);
        count.set(2);
    });

    let runs = effect_runs.borrow();
    assert_eq!(runs.len(), 3);
    assert_eq!(runs[0], (0, Theme(String::from("dark"))));
    assert_eq!(runs[1], (1, Theme(String::from("dark"))));
    assert_eq!(runs[2], (2, Theme(String::from("dark"))));
}

#[test]
fn test_integration_context_with_computed() {
    scope(|| {
        provide_context(Config(10));
        let multiplier = signal(2);

        let doubled = computed(move |_| {
            let config = use_context::<Config>().unwrap();
            config.0 * multiplier.get()
        });

        assert_eq!(doubled.get(), 20);
        multiplier.set(3);
        assert_eq!(doubled.get(), 30);
    });
}

#[test]
fn test_integration_deeply_nested_scopes() {
    scope(|| {
        provide_context(Theme(String::from("root")));
        provide_context(Config(100));

        scope(|| {
            // Level 2: can access both contexts
            assert_eq!(use_context::<Theme>().unwrap(), Theme(String::from("root")));
            assert_eq!(use_context::<Config>().unwrap(), Config(100));

            scope(|| {
                // Level 3: shadow Config
                provide_context(Config(200));
                assert_eq!(use_context::<Theme>().unwrap(), Theme(String::from("root")));
                assert_eq!(use_context::<Config>().unwrap(), Config(200));

                scope(|| {
                    // Level 4: shadow Theme
                    provide_context(Theme(String::from("nested")));
                    assert_eq!(
                        use_context::<Theme>().unwrap(),
                        Theme(String::from("nested"))
                    );
                    assert_eq!(use_context::<Config>().unwrap(), Config(200));
                });

                assert_eq!(use_context::<Theme>().unwrap(), Theme(String::from("root")));
                assert_eq!(use_context::<Config>().unwrap(), Config(200));
            });

            assert_eq!(use_context::<Theme>().unwrap(), Theme(String::from("root")));
            assert_eq!(use_context::<Config>().unwrap(), Config(100));
        });
    });
}

#[test]
fn test_integration_context_cleanup_on_scope_disposal() {
    scope(|| {
        provide_context(Config(42));
        assert_eq!(use_context::<Config>().unwrap(), Config(42));
    });

    // After scope is disposed, context should not be accessible
    assert!(use_context::<Config>().is_none());
}

#[test]
fn test_scoped_basic_context() {
    scope(|| {
        provide_context(Theme(String::from("dark")));

        // Create scoped closure that captures current scope
        let scoped_fn = scoped(|_: ()| {
            // Can access context from parent scope captured at creation time
            let ctx = use_context::<Theme>().unwrap();
            ctx
        });

        let (context, _) = scoped_fn(());
        assert_eq!(context, Theme(String::from("dark")));

        provide_context(Theme(String::from("light")));

        let (context, _) = scoped_fn(());
        assert_eq!(context, Theme(String::from("light")));
    });
}
