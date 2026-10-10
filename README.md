# samara-signals

A high-performance reactive signal library for Rust, porting the
[alien-signals](https://github.com/evanryu/alien-signals) propagation algorithm:
link/version-based dependency tracking, glitch-free updates, and equality
short-circuiting for derived values — with no garbage collector and no
per-node reference counting.

```rust
use samara_signals::*;

let src = signal(1);

// Derived values recompute lazily and only propagate on change
let doubled = memo(move || src.get() * 2);

let log = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
let log2 = log.clone();
let _effect = effect(move || {
    log2.borrow_mut().push(doubled.get());
});

assert_eq!(*log.borrow(), vec![2]);

src.set(10); // effect re-runs once, memo recomputed once
assert_eq!(*log.borrow(), vec![2, 20]);
```

## What's inside

- **`signal`** — reactive state with guards (`read`/`write`/`peek`), `with`/`update` callbacks
- **`computed` / `memo`** — lazily evaluated derived values; `memo` only propagates when the value actually changes (`PartialEq`)
- **`effect` / `scope`** — automatic dependency tracking and hierarchical cleanup; children created inside an effect are disposed and re-created on each run
- **context** — typed, scope-aware dependency injection (`provide_context` / `use_context`)
- **`resource`** — async values that re-fetch whenever signals read in the fetch closure change; in-flight fetches are aborted on refetch

## Guarantees

- **Glitch-free**: diamond dependencies settle before effects run; effects observe consistent values
- **Misuse panics instead of corrupting memory**: use-after-dispose, signal borrow conflicts, and dependency cycles are detected with explicit panic messages in debug *and* release
- Debug builds additionally enforce the reactive system's borrow discipline with caller locations; the core test suites are clean under Miri

## Scopes and disposal

Everything created inside a `scope` (signals, memos, effects, nested scopes) is
cleaned up on `dispose` — links unlinked, callbacks freed. Handles are `Copy`
and not lifetime-branded; using one after disposal panics.

```rust
let scope = scope(|| {
    let s = signal(1);
    let _m = memo(move || s.get() + 1);
});
scope.dispose();
```

## Async

`spawn` runs futures on a single-threaded executor that preserves the reactive
context across `await` points; `join().await` drains all tasks.

```rust
let user_id = signal(1);
let r = resource(move || {
    let id = user_id.get(); // tracked: refetch on change
    async move { fetch_user(id).await }
});
join().await;
```

## Benchmarks

`cargo bench --bench propagate` measures wide/deep memo graphs (1×1 up to
1000×1000 nodes). Steady state is ~25ns per memo update — flat from
cache-resident to DRAM-resident graph sizes.

## License

MIT
