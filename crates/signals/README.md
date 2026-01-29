# alien-signals (Rust)

A high-performance, push-pull based reactive signals library for Rust. This is the Rust implementation of [alien-signals](https://github.com/stackblitz/alien-signals), featuring the same core algorithm optimized for Rust's ownership model and zero-cost abstractions.

## Features

- **High Performance**: Push-pull propagation algorithm with minimal overhead
- **Type Safety**: Leverages Rust's type system for compile-time guarantees
- **Memory Efficient**: Uses `SlotMap` for stable node references and cache-friendly operations
- **Effect Scopes**: Create isolated scopes with automatic cleanup via `scope()` and `scoped()`
- **Nested Effects**: Full support for nested effect scopes with automatic cleanup
- **Fine-grained Reactivity**: Track dependencies at the expression level
- **Manual Batching**: Control when effects run with `start_batch`/`end_batch`

## Performance

Benchmark results comparing Rust implementation to TypeScript version:

### Typescript

```md
> alien-signals@3.1.2 bench /Users/wuzekang/repos/alien-signals
> npm run build && node --expose-gc benchs/propagate.mjs

> alien-signals@3.1.2 build
> node ./build.js

clk: ~3.14 GHz
cpu: Apple M2 Pro
runtime: node 22.17.0 (arm64-darwin)

| benchmark              |              avg |         min |         p75 |         p99 |         max |
| ---------------------- | ---------------- | ----------- | ----------- | ----------- | ----------- |
| propagate: 1 * 1       | ` 41.10 ns/iter` | ` 38.76 ns` | ` 41.62 ns` | ` 46.65 ns` | ` 74.98 ns` |
| propagate: 1 * 10      | `222.95 ns/iter` | `209.74 ns` | `225.85 ns` | `239.19 ns` | `245.38 ns` |
| propagate: 1 * 100     | `  1.92 µs/iter` | `  1.88 µs` | `  1.94 µs` | `  1.97 µs` | `  1.98 µs` |
| propagate: 1 * 1000    | ` 31.47 µs/iter` | ` 31.11 µs` | ` 31.72 µs` | ` 31.94 µs` | ` 31.99 µs` |
| propagate: 10 * 1      | `408.79 ns/iter` | `391.58 ns` | `414.05 ns` | `426.69 ns` | `438.91 ns` |
| propagate: 10 * 10     | `  2.01 µs/iter` | `  1.98 µs` | `  2.02 µs` | `  2.07 µs` | `  2.07 µs` |
| propagate: 10 * 100    | ` 34.44 µs/iter` | ` 33.53 µs` | ` 34.40 µs` | ` 35.12 µs` | ` 38.81 µs` |
| propagate: 10 * 1000   | `345.75 µs/iter` | `321.96 µs` | `345.54 µs` | `444.75 µs` | `863.17 µs` |
| propagate: 100 * 1     | `  3.89 µs/iter` | `  3.83 µs` | `  3.93 µs` | `  3.98 µs` | `  3.98 µs` |
| propagate: 100 * 10    | ` 25.10 µs/iter` | ` 24.92 µs` | ` 25.18 µs` | ` 25.45 µs` | ` 25.48 µs` |
| propagate: 100 * 100   | `245.28 µs/iter` | `226.50 µs` | `248.92 µs` | `307.63 µs` | `423.29 µs` |
| propagate: 100 * 1000  | `  6.35 ms/iter` | `  5.54 ms` | `  6.51 ms` | `  8.65 ms` | `  9.61 ms` |
| propagate: 1000 * 1    | ` 48.65 µs/iter` | ` 48.16 µs` | ` 48.70 µs` | ` 49.11 µs` | ` 49.25 µs` |
| propagate: 1000 * 10   | `298.92 µs/iter` | `276.00 µs` | `303.46 µs` | `364.58 µs` | `448.17 µs` |
| propagate: 1000 * 100  | `  3.67 ms/iter` | `  3.29 ms` | `  3.72 ms` | `  4.98 ms` | `  5.31 ms` |
| propagate: 1000 * 1000 | `137.82 ms/iter` | `136.53 ms` | `138.51 ms` | `138.98 ms` | `139.59 ms` |
```

### Rust (Criterion)

```md
1 * 1                   time:   [42.656 ns 42.974 ns 43.331 ns]
1 * 10                  time:   [248.55 ns 249.21 ns 249.91 ns]
1 * 100                 time:   [1.8878 µs 1.9029 µs 1.9230 µs]
1 * 1000                time:   [21.002 µs 21.146 µs 21.303 µs]
10 * 1                  time:   [356.37 ns 359.19 ns 362.78 ns]
10 * 10                 time:   [2.1833 µs 2.1900 µs 2.1972 µs]
10 * 100                time:   [23.719 µs 23.789 µs 23.862 µs]
10 * 1000               time:   [227.03 µs 227.59 µs 228.17 µs]
100 * 1                 time:   [3.5748 µs 3.5856 µs 3.5973 µs]
100 * 10                time:   [25.508 µs 25.580 µs 25.660 µs]
100 * 100               time:   [250.30 µs 251.80 µs 253.55 µs]
100 * 1000              time:   [2.2859 ms 2.3053 ms 2.3277 ms]
1000 * 1                time:   [40.681 µs 40.884 µs 41.202 µs]
1000 * 10               time:   [251.59 µs 252.23 µs 252.90 µs]
1000 * 100              time:   [2.7178 ms 2.7508 ms 2.7872 ms]
1000 * 1000             time:   [24.085 ms 24.332 ms 24.679 ms]
```

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
samara_signals = "0.1"
```

## Usage

### Signals, Computed, and Effects

```rust
use samara_signals::{signal, computed, effect};

let count = signal(1);
let double_count = computed(move || count.get() * 2);

let _effect = effect(move || {
    println!("Count is: {}", count.get());
});
// Console: Count is: 1

println!("{}", double_count.get()); // 2

count.set(2);
// Console: Count is: 2

println!("{}", double_count.get()); // 4
```

### Reading and Writing Signals

The library provides different methods for accessing signal values:

```rust
use samara_signals::signal;

let s = signal(vec![1, 2, 3]);

// Get a copy of the value
let value = s.get();

// Get a reference with automatic tracking
let guard = s.read();
println!("{:?}", guard.len());
// guard is dropped here, untracking the signal

// Get a mutable reference without triggering updates
let mut guard = s.write();
guard.push(4);
// Changes are applied but effects are not notified
```

### Effect Scopes

Scopes allow you to group effects and clean them up together:

```rust
use samara_signals::{signal, effect, scope};

let count = signal(1);

let scope = scope(move || {
    effect(move || {
        println!("Count in scope: {}", count.get());
    });
});

count.set(2);

scope.cleanup();

count.set(3);
// panic - effect was cleaned up
```

### Nested Effects

Effects can be nested inside other effects. Inner effects from previous runs are automatically cleaned up:

```rust
use samara_signals::{signal, effect};

let show = signal(true);
let count = signal(1);

effect(move || {
    if show.get() {
        // Inner effect is created when show() is true
        effect(move || {
            println!("Count is: {}", count.get());
        });
    }
});
// Console: Count is: 1

count.set(2);
// Console: Count is: 2

// When show becomes false, inner effect is cleaned up
show.set(false);
// No output

count.set(3);
// No output (inner effect no longer exists)
```

### Scoped Utility

Create reusable scoped computations with deferred execution:

```rust
use samara_signals::scoped;

let scoped_fn = scoped(|x: i32| {
    let s = signal(x);
    s.get() * 2
});

// Execute multiple times with cleanup
let (result1, scope1) = scoped_fn(5);
println!("{}", result1); // 10

let (result2, scope2) = scoped_fn(10);
println!("{}", result2); // 20

// Cleanup when done
scope1.cleanup();
scope2.cleanup();
```

### Manual Batching

Control when effects are executed:

```rust
use samara_signals::{signal, effect, start_batch, end_batch};

let s = signal(0);
let mut count = 0;

let _effect = effect(move || {
    count += 1;
    s.get();
});

start_batch();
s.set(1);
s.set(2);
s.set(3);
end_batch();

// Effect only runs once
assert_eq!(count, 2);
```

### Effect Cleanup Callbacks

```rust
use samara_signals::{signal, effect, on_cleanup};

let s = signal(1);
let _effect = effect(move || {
    on_cleanup(|| {
        println!("Effect is being cleaned up");
    });
    s.get();
});
```

### Borrow Checking

The library provides runtime borrow checking to prevent memory issues:

```rust
use samara_signals::{signal};

let s = signal(vec![1]);

// Reading is safe
let guard1 = s.read();
let len = guard1.len();

// Multiple reads are allowed
let guard2 = s.read();
let first = guard1[0];

// But writing while reading would panic at runtime
// let mut guard3 = s.write(); // This would panic!
```

## Testing

Run the test suite:

```bash
cargo test
```

Run benchmarks:

```bash
cargo bench
```

## License

MIT
