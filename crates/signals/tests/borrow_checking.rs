use samara_signals::*;

// Valid patterns - should NOT panic
#[test]
fn test_multiple_concurrent_read_guards() {
    let s = signal(42i32);
    let guard1 = s.read();
    let guard2 = s.read();
    let guard3 = s.read();
    assert_eq!(*guard1, 42);
    assert_eq!(*guard2, 42);
    assert_eq!(*guard3, 42);
}

#[test]
fn test_sequential_read_and_write() {
    let s = signal(42i32);
    {
        let guard = s.read();
        assert_eq!(*guard, 42);
    }
    {
        let mut guard = s.write();
        *guard = 100;
    }
    assert_eq!(s.get(), 100);
}

#[test]
fn test_guard_drop_releases_borrow() {
    let s = signal(42i32);
    {
        let _guard = s.write();
    }
    let _guard2 = s.read(); // Should work after drop
}

// Panic scenarios - SHOULD panic
#[test]
#[should_panic]
fn test_write_then_read_panics() {
    let s = signal(42i32);
    let _write_guard = s.write();
    let _read_guard = s.read(); // Panic
}

#[test]
#[should_panic]
fn test_read_then_write_panics() {
    let s = signal(42i32);
    let _read_guard = s.read();
    let _write_guard = s.write(); // Panic
}

#[test]
#[should_panic]
fn test_multiple_write_guards_panics() {
    let s = signal(42i32);
    let _guard1 = s.write();
    let _guard2 = s.write(); // Panic
}

#[test]
#[should_panic]
fn test_write_guard_prevents_peek() {
    let s = signal(42i32);
    let _write_guard = s.write();
    let _ = s.peek(); // Panic
}

#[test]
#[should_panic]
fn test_read_guard_prevents_set() {
    let s = signal(42i32);
    let _read_guard = s.read();
    s.set(100); // Panic - cannot set while reading
}

#[test]
#[should_panic]
fn test_write_guard_prevents_set() {
    let s = signal(42i32);
    let _write_guard = s.write();
    s.set(100); // Panic - cannot set while writing
}
