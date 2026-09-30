use super::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[derive(Clone)]
struct Count(Rc<Cell<usize>>);

impl Count {
    fn new() -> Self {
        Self(Rc::new(Cell::new(0)))
    }

    fn inc(&self) {
        self.0.update(|c| c + 1);
    }

    fn get(&self) -> usize {
        self.0.get()
    }

    fn reset(&self) {
        self.0.set(0);
    }
}

#[test]
fn contexts() {
    create_scope().run_in(|| {
        provide_context(12);

        assert_eq!(use_context::<i32>(), 12);

        create_scope().run_in(|| {
            provide_context(-12);
            assert_eq!(use_context::<i32>(), -12);
        });
    });
}

#[test]
fn context_nesting() {
    let scope = create_scope();

    scope.run_in(|| {
        provide_context(String::from("parent"));
        assert_eq!(use_context::<String>(), "parent");

        create_scope().run_in(|| {
            provide_context(String::from("child"));
            assert_eq!(use_context::<String>(), "child");
        });

        assert_eq!(use_context::<String>(), "parent");
    });
}

#[test]
fn tripple_count() {
    let scope = create_scope();

    let (hi, child) = scope.run_in(|| {
        let hi = create_signal("hi");
        let mut count = create_signal(0);

        create_effect(move || {
            dbg!(hi());
            count += 1;
        });

        let double_count = create_memo(move || count * 2);

        let child = create_scope();

        let tripple_count = child.run_in(|| {
            on_cleanup(|| dbg!("clean!"));
            create_memo(move || count * 3)
        });

        create_effect(move || {
            assert_eq!(dbg!(count) * 2, dbg!(double_count()));
            dbg!(tripple_count());
        });

        (hi, child)
    });

    flush();

    hi.set("g");
    flush();

    hi.set("c");
    flush();

    drop(child);
    flush();
}

#[test]
fn memo_chain() {
    let scope = create_scope();

    let (source, runs, values) = scope.run_in(|| {
        let source = create_signal(1);
        let runs = Count::new();
        let values = Rc::new(RefCell::new(Vec::new()));

        let doubled = create_memo(move || source() * 2);
        let result = create_memo(move || doubled() * 2);

        create_effect({
            let runs = runs.clone();
            let values = values.clone();

            move || {
                runs.inc();
                values.borrow_mut().push(result());
            }
        });

        (source, runs, values)
    });

    flush();
    runs.reset();
    values.borrow_mut().clear();

    source.set(2);
    flush();

    assert_eq!(runs.get(), 1);
    assert_eq!(&*values.borrow(), &[8]);
}

#[test]
fn deep() {
    const N: usize = 5_000;

    let scope = create_scope();

    let (source, runs) = scope.run_in(|| {
        let source = create_signal(0);
        let runs = Count::new();
        let mut previous = source;

        for _ in 0..N {
            let next = create_signal(0);

            create_effect({
                let runs = runs.clone();

                move || {
                    let value = previous();
                    runs.inc();
                    next.set(value + 1);
                }
            });

            previous = next;
        }

        (source, runs)
    });

    flush();
    runs.reset();

    source.set(1);
    flush();

    assert_eq!(runs.get(), N);
}

#[test]
fn diamond() {
    let scope = create_scope();

    let (source, runs, values) = scope.run_in(|| {
        let source = create_signal(1);
        let runs = Count::new();
        let values = Rc::new(RefCell::new(Vec::new()));

        let left = create_memo(move || source() + 1);

        let right = create_memo(move || source() * 2);

        let result = create_memo(move || left() + right());

        create_effect({
            let runs = runs.clone();
            let values = values.clone();

            move || {
                runs.inc();
                values.borrow_mut().push(result());
            }
        });

        (source, runs, values)
    });

    flush();
    runs.reset();
    values.borrow_mut().clear();

    source.set(2);
    flush();

    assert_eq!(runs.get(), 1);
    assert_eq!(&*values.borrow(), &[7]);
}

#[test]
fn wide_diamond() {
    let scope = create_scope();

    let (source, runs, values) = scope.run_in(|| {
        let source = create_signal(0);
        let runs = Count::new();
        let values = Rc::new(RefCell::new(Vec::new()));

        let a = create_memo(move || source() + 1);

        let b = create_memo(move || source() + 2);

        let c = create_memo(move || source() + 3);

        let d = create_memo(move || source() + 4);

        let ab = create_memo(move || a() + b());
        let cd = create_memo(move || c() + d());

        let result = create_memo(move || ab() + cd());

        create_effect({
            let runs = runs.clone();
            let values = values.clone();

            move || {
                runs.inc();
                values.borrow_mut().push(result());
            }
        });

        (source, runs, values)
    });

    flush();
    runs.reset();
    values.borrow_mut().clear();

    source.set(1);
    flush();

    assert_eq!(runs.get(), 1);
    assert_eq!(&*values.borrow(), &[14]);
}

#[test]
fn fan_out() {
    let scope = create_scope();

    let (source, runs, values) = scope.run_in(|| {
        let source = create_signal(1);
        let runs = Count::new();
        let values = Rc::new(RefCell::new(Vec::new()));

        let a = create_memo(move || source() + 1);

        let b = create_memo(move || source() + 2);

        let c = create_memo(move || source() + 3);

        create_effect({
            let runs = runs.clone();
            let values = values.clone();

            move || {
                runs.inc();
                values.borrow_mut().push((a(), b(), c()));
            }
        });

        (source, runs, values)
    });

    flush();
    runs.reset();
    values.borrow_mut().clear();

    source.set(10);
    flush();

    assert_eq!(runs.get(), 1);
    assert_eq!(&*values.borrow(), &[(11, 12, 13)]);
}

#[test]
fn batched() {
    let scope = create_scope();

    let (a, b, c, runs, values) = scope.run_in(|| {
        let a = create_signal(0);
        let b = create_signal(0);
        let c = create_signal(0);

        let sum = create_memo(move || a() + b() + c());
        let result = create_memo(move || sum() * 2);

        let runs = Count::new();
        let values = Rc::new(RefCell::new(Vec::new()));

        create_effect({
            let runs = runs.clone();
            let values = values.clone();

            move || {
                runs.inc();
                values.borrow_mut().push(result());
            }
        });

        (a, b, c, runs, values)
    });

    flush();
    runs.reset();
    values.borrow_mut().clear();

    a.set(1);
    b.set(2);
    c.set(3);

    flush();

    assert_eq!(runs.get(), 1);
    assert_eq!(&*values.borrow(), &[12]);
}

#[test]
fn multiple_writes_same_signal_before_flush() {
    let scope = create_scope();

    let (source, runs, values) = scope.run_in(|| {
        let source = create_signal(0);
        let runs = Count::new();
        let values = Rc::new(RefCell::new(Vec::new()));

        create_effect({
            let runs = runs.clone();
            let values = values.clone();

            move || {
                runs.inc();
                values.borrow_mut().push(source());
            }
        });

        (source, runs, values)
    });

    flush();
    runs.reset();
    values.borrow_mut().clear();

    source.set(1);
    source.set(2);
    source.set(3);

    flush();

    assert_eq!(runs.get(), 1);
    assert_eq!(&*values.borrow(), &[3]);
}

#[test]
fn independent_memos() {
    let scope = create_scope();

    let (a, b, runs, values) = scope.run_in(|| {
        let a = create_signal(1);
        let b = create_signal(2);

        let sum = create_memo(move || a() + b());
        let product = create_memo(move || a() * b());

        let runs = Count::new();
        let values = Rc::new(RefCell::new(Vec::new()));

        create_effect({
            let runs = runs.clone();
            let values = values.clone();

            move || {
                runs.inc();
                values.borrow_mut().push((sum(), product()));
            }
        });

        (a, b, runs, values)
    });

    flush();
    runs.reset();
    values.borrow_mut().clear();

    a.set(3);
    b.set(4);

    flush();

    assert_eq!(runs.get(), 1);
    assert_eq!(&*values.borrow(), &[(7, 12)]);
}

#[test]
fn cascading_writes() {
    let scope = create_scope();

    let (input, runs, values) = scope.run_in(|| {
        let input = create_signal(0);
        let intermediate = create_signal(0);

        let runs = Count::new();
        let values = Rc::new(RefCell::new(Vec::new()));

        create_effect(move || {
            input.track();
            intermediate.set(input() * 2);
        });

        create_effect({
            let runs = runs.clone();
            let values = values.clone();

            move || {
                runs.inc();
                values.borrow_mut().push(intermediate());
            }
        });

        (input, runs, values)
    });

    flush();
    runs.reset();
    values.borrow_mut().clear();

    input.set(10);
    flush();

    assert_eq!(runs.get(), 1);
    assert_eq!(&*values.borrow(), &[20]);
}

#[test]
fn no_glitch_in_memo_chain() {
    let scope = create_scope();

    let (source, values) = scope.run_in(|| {
        let source = create_signal(1);
        let values = Rc::new(RefCell::new(Vec::new()));

        let a = create_memo(move || source() * 2);
        let b = create_memo(move || a() + 1);
        let c = create_memo(move || b() * 2);

        create_effect({
            let values = values.clone();

            move || {
                values.borrow_mut().push((a(), b(), c()));
            }
        });

        (source, values)
    });

    flush();
    values.borrow_mut().clear();

    source.set(2);
    flush();

    assert_eq!(&*values.borrow(), &[(4, 5, 10)]);
}

#[test]
fn reading_same_memo_multiple_times() {
    let scope = create_scope();

    let (source, runs) = scope.run_in(|| {
        let source = create_signal(0);
        let runs = Count::new();

        let memo = create_memo(move || source() + 1);

        create_effect({
            let runs = runs.clone();

            move || {
                let _ = memo();
                let _ = memo();
                let _ = memo();

                runs.inc();
            }
        });

        (source, runs)
    });

    flush();
    runs.reset();

    source.set(1);
    flush();

    assert_eq!(runs.get(), 1);
}

#[test]
fn memo_computes_once_per_update() {
    let scope = create_scope();

    let (source, computations) = scope.run_in(|| {
        let source = create_signal(0);
        let computations = Count::new();

        let memo = create_memo({
            let computations = computations.clone();

            move || {
                computations.inc();
                source()
            }
        });

        create_effect(move || {
            memo();
            memo();
            memo();
        });

        (source, computations)
    });

    flush();
    computations.reset();

    source.set(1);
    flush();

    assert_eq!(computations.get(), 1);
}

#[test]
fn selector_equal_value_does_not_notify() {
    let scope = create_scope();

    let (source, runs) = scope.run_in(|| {
        let source = create_signal(0);
        let runs = Count::new();

        let selected = create_selector(move || source() % 2);

        create_effect({
            let runs = runs.clone();

            move || {
                selected.track();
                runs.inc();
            }
        });

        (source, runs)
    });

    flush();
    runs.reset();

    source.set(2);
    flush();

    assert_eq!(runs.get(), 0);

    source.set(3);
    flush();

    assert_eq!(runs.get(), 1);
}

#[test]
fn selector_changes_value() {
    let scope = create_scope();

    let (source, runs, values) = scope.run_in(|| {
        let source = create_signal(1);
        let runs = Count::new();
        let values = Rc::new(RefCell::new(Vec::new()));

        let selected = create_selector(move || source());

        create_effect({
            let runs = runs.clone();
            let values = values.clone();

            move || {
                runs.inc();
                values.borrow_mut().push(selected());
            }
        });

        (source, runs, values)
    });

    flush();
    runs.reset();
    values.borrow_mut().clear();

    source.set(2);
    flush();

    assert_eq!(runs.get(), 1);
    assert_eq!(&*values.borrow(), &[2]);
}

#[test]
fn selector_dynamic_dependencies() {
    let scope = create_scope();

    let (condition, left, right, runs) = scope.run_in(|| {
        let condition = create_signal(true);
        let left = create_signal(10);
        let right = create_signal(20);
        let runs = Count::new();

        let selected = create_selector(move || if condition() { left() } else { right() });

        create_effect({
            let runs = runs.clone();

            move || {
                selected.track();
                runs.inc();
            }
        });

        (condition, left, right, runs)
    });

    flush();
    runs.reset();

    left.set(11);
    flush();

    assert_eq!(runs.get(), 1);

    runs.reset();

    condition.set(false);
    flush();

    assert_eq!(runs.get(), 1);

    runs.reset();

    left.set(12);
    flush();

    assert_eq!(runs.get(), 0);

    right.set(21);
    flush();

    assert_eq!(runs.get(), 1);
}

#[test]
fn selector_switching_dependencies() {
    let scope = create_scope();

    let (condition, left, right, runs) = scope.run_in(|| {
        let condition = create_signal(true);
        let left = create_signal(0);
        let right = create_signal(0);
        let runs = Count::new();

        let selected = create_selector(move || if condition() { left() } else { right() });

        create_effect({
            let runs = runs.clone();

            move || {
                selected.track();
                runs.inc();
            }
        });

        (condition, left, right, runs)
    });

    flush();
    runs.reset();

    left.set(1);
    flush();
    assert_eq!(runs.get(), 1);

    runs.reset();

    condition.set(false);
    flush();
    assert_eq!(runs.get(), 1);

    runs.reset();

    left.set(2);
    flush();
    assert_eq!(runs.get(), 0);

    right.set(1);
    flush();
    assert_eq!(runs.get(), 1);

    runs.reset();

    condition.set(true);
    flush();
    assert_eq!(runs.get(), 1);

    runs.reset();

    right.set(2);
    flush();
    assert_eq!(runs.get(), 0);

    left.set(3);
    flush();
    assert_eq!(runs.get(), 1);
}

#[test]
fn selector_equal_value_after_dynamic_switch() {
    let scope = create_scope();

    let (condition, left, right, runs, values) = scope.run_in(|| {
        let condition = create_signal(true);
        let left = create_signal(10);
        let right = create_signal(10);
        let runs = Count::new();
        let values = Rc::new(RefCell::new(Vec::new()));

        let selected = create_selector(move || if condition() { left() } else { right() });

        create_effect({
            let runs = runs.clone();
            let values = values.clone();

            move || {
                selected.track();
                runs.inc();
                values.borrow_mut().push(selected());
            }
        });

        (condition, left, right, runs, values)
    });

    flush();
    runs.reset();
    values.borrow_mut().clear();

    condition.set(false);
    flush();

    assert_eq!(runs.get(), 0);
    assert!(values.borrow().is_empty());

    right.set(11);
    flush();

    assert_eq!(runs.get(), 1);
    assert_eq!(&*values.borrow(), &[11]);

    runs.reset();
    values.borrow_mut().clear();

    condition.set(true);
    flush();

    assert_eq!(runs.get(), 1);
    assert_eq!(&*values.borrow(), &[10]);

    runs.reset();
    values.borrow_mut().clear();

    left.set(10);
    flush();

    assert_eq!(runs.get(), 0);
    assert!(values.borrow().is_empty());
}

#[test]
fn selector_diamond() {
    let scope = create_scope();

    let (source, runs, values) = scope.run_in(|| {
        let source = create_signal(0);
        let runs = Count::new();
        let values = Rc::new(RefCell::new(Vec::new()));

        let left = create_memo(move || source() % 2);

        let right = create_memo(move || 10 - (source() % 2));

        let selected = create_selector(move || left() + right());

        create_effect({
            let runs = runs.clone();
            let values = values.clone();

            move || {
                selected.track();
                runs.inc();
                values.borrow_mut().push(selected());
            }
        });

        (source, runs, values)
    });

    flush();
    runs.reset();
    values.borrow_mut().clear();

    source.set(1);
    flush();

    assert_eq!(runs.get(), 0);
    assert!(values.borrow().is_empty());

    source.set(2);
    flush();

    assert_eq!(runs.get(), 0);
    assert!(values.borrow().is_empty());
}

#[test]
fn selector_batched_updates() {
    let scope = create_scope();

    let (source, runs, values) = scope.run_in(|| {
        let source = create_signal(0);
        let runs = Count::new();
        let values = Rc::new(RefCell::new(Vec::new()));

        let selected = create_selector(move || source() % 2);

        create_effect({
            let runs = runs.clone();
            let values = values.clone();

            move || {
                selected.track();
                runs.inc();
                values.borrow_mut().push(selected());
            }
        });

        (source, runs, values)
    });

    flush();
    runs.reset();
    values.borrow_mut().clear();

    source.set(1);
    source.set(2);
    source.set(3);

    flush();

    assert_eq!(runs.get(), 1);
    assert_eq!(&*values.borrow(), &[1]);
}

#[test]
fn selector_multiple_subscribers() {
    let scope = create_scope();

    let (source, runs_a, runs_b) = scope.run_in(|| {
        let source = create_signal(0);
        let runs_a = Count::new();
        let runs_b = Count::new();

        let selected = create_selector(move || source() % 2);

        create_effect({
            let runs = runs_a.clone();

            move || {
                selected.track();
                runs.inc();
            }
        });

        create_effect({
            let runs = runs_b.clone();

            move || {
                selected.track();
                runs.inc();
            }
        });

        (source, runs_a, runs_b)
    });

    flush();

    runs_a.reset();
    runs_b.reset();

    source.set(2);
    flush();

    assert_eq!(runs_a.get(), 0);
    assert_eq!(runs_b.get(), 0);

    source.set(3);
    flush();

    assert_eq!(runs_a.get(), 1);
    assert_eq!(runs_b.get(), 1);
}

#[test]
fn scope_cleanup() {
    let scope = create_scope();
    let cleanup_count = Rc::new(Cell::new(0));

    let (source, child, outer_runs, inner_runs) = scope.run_in(|| {
        let source = create_signal(0);
        let child = create_scope();

        let outer_runs = Count::new();
        let inner_runs = Count::new();

        create_effect({
            let outer_runs = outer_runs.clone();

            move || {
                source.track();
                outer_runs.inc();
            }
        });

        child.run_in({
            let inner_runs = inner_runs.clone();
            let cleanup_count = cleanup_count.clone();

            move || {
                on_cleanup(move || {
                    cleanup_count.set(cleanup_count.get() + 1);
                });

                create_effect(move || {
                    source.track();
                    inner_runs.inc();
                });
            }
        });

        (source, child, outer_runs, inner_runs)
    });

    flush();

    outer_runs.reset();
    inner_runs.reset();

    drop(child);

    assert_eq!(cleanup_count.get(), 1);

    source.set(1);
    flush();

    assert_eq!(outer_runs.get(), 1);
    assert_eq!(inner_runs.get(), 0);
}

#[test]
fn nested_scope_cleanup() {
    let scope = create_scope();
    let cleanup_count = Rc::new(Cell::new(0));

    let (source, child, outer, inner, deepest) = scope.run_in(|| {
        let source = create_signal(0);

        let outer = Count::new();
        let inner = Count::new();
        let deepest = Count::new();

        create_effect({
            let outer = outer.clone();

            move || {
                source.track();
                outer.inc();
            }
        });

        let child = create_scope();

        child.run_in({
            let inner = inner.clone();
            let deepest = deepest.clone();
            let cleanup_count = cleanup_count.clone();

            move || {
                on_cleanup({
                    let cleanup_count = cleanup_count.clone();

                    move || {
                        cleanup_count.set(cleanup_count.get() + 1);
                    }
                });

                create_effect({
                    let inner = inner.clone();

                    move || {
                        source.track();
                        inner.inc();
                    }
                });

                let grandchild = create_scope();

                grandchild.run_in({
                    let deepest = deepest.clone();

                    move || {
                        create_effect({
                            let deepest = deepest.clone();

                            move || {
                                source.track();
                                deepest.inc();
                            }
                        });
                    }
                });
            }
        });

        (source, child, outer, inner, deepest)
    });

    flush();

    outer.reset();
    inner.reset();
    deepest.reset();

    drop(child);

    assert_eq!(cleanup_count.get(), 1);

    source.set(1);
    flush();

    assert_eq!(outer.get(), 1);
    assert_eq!(inner.get(), 0);
    assert_eq!(deepest.get(), 0);
}

#[test]
fn flush_is_idempotent() {
    let scope = create_scope();

    let (signal, runs) = scope.run_in(|| {
        let signal = create_signal(0);
        let runs = Count::new();

        create_effect({
            let runs = runs.clone();

            move || {
                signal.track();
                runs.inc();
            }
        });

        (signal, runs)
    });

    flush();
    runs.reset();

    for _ in 0..10 {
        flush();
    }

    assert_eq!(runs.get(), 0);

    signal.set(1);
    flush();

    assert_eq!(runs.get(), 1);

    for _ in 0..10 {
        flush();
    }

    assert_eq!(runs.get(), 1);
}
