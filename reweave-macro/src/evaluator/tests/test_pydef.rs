mod pydef_tests {
    use crate::evaluator::{EvalConfig, Evaluator, eval_string};

    fn evaluator() -> Evaluator {
        Evaluator::new(EvalConfig::default())
    }

    #[test]
    fn test_double() {
        let mut ev = evaluator();
        let src = r#"%pydef(double, x, %{str(int(x) * 2)%})
%double(21)"#;
        let result = eval_string(src, None, &mut ev).expect("eval failed");
        assert_eq!(result.trim(), "42");
    }

    #[test]
    fn test_offset() {
        let mut ev = evaluator();
        let src = r#"%pydef(offset, base, size, %{
str(int(base) + int(size))
%})
%offset(256, 64)"#;
        let result = eval_string(src, None, &mut ev).expect("eval failed");
        assert_eq!(result.trim(), "320");
    }

    #[test]
    fn test_greet() {
        let mut ev = evaluator();
        let src = r#"%pydef(greet, name, %{
"Hello, " + name + "!"
%})
%greet(world)"#;
        let result = eval_string(src, None, &mut ev).expect("eval failed");
        assert_eq!(result.trim(), "Hello, world!");
    }

    #[test]
    fn test_only_declared_params_visible() {
        let mut ev = evaluator();
        let src = r#"%set(secret, hidden)
%pydef(echo, x, %{x%})
%echo(visible)"#;
        let result = eval_string(src, None, &mut ev).expect("eval failed");
        assert_eq!(result.trim(), "visible");
    }

    #[test]
    fn test_error_propagation() {
        let mut ev = evaluator();
        let src = r#"%pydef(broken, x, %{1 / 0%})
%broken(foo)"#;
        let result = eval_string(src, None, &mut ev);
        assert!(result.is_err(), "expected error from division by zero");
    }

    // --- store tests ---

    #[test]
    fn test_pyset_pyget_roundtrip() {
        let mut ev = evaluator();
        let src = r#"%pyset(color, red)
%pyget(color)"#;
        let result = eval_string(src, None, &mut ev).expect("eval failed");
        assert_eq!(result.trim(), "red");
    }

    #[test]
    fn test_store_persists_across_calls() {
        let mut ev = evaluator();
        let src = r#"%pyset(counter, 0)
%pydef(increment, %{str(int(counter) + 1)%})
%pyset(counter, %increment())
%pyset(counter, %increment())
%pyget(counter)"#;
        let result = eval_string(src, None, &mut ev).expect("eval failed");
        assert_eq!(result.trim(), "2");
    }

    #[test]
    fn test_store_visible_in_script() {
        let mut ev = evaluator();
        let src = r#"%pyset(prefix, item_)
%pydef(tagged, name, %{prefix + name%})
%tagged(count)"#;
        let result = eval_string(src, None, &mut ev).expect("eval failed");
        assert_eq!(result.trim(), "item_count");
    }

    #[test]
    fn test_param_shadows_store_key() {
        let mut ev = evaluator();
        let src = r#"%pyset(x, store_value)
%pydef(identity, x, %{x%})
%identity(param_value)"#;
        let result = eval_string(src, None, &mut ev).expect("eval failed");
        assert_eq!(result.trim(), "param_value");
    }

    #[test]
    fn test_running_sum() {
        let mut ev = evaluator();
        let src = r#"%pyset(total, 0)
%pydef(add, n, %{str(int(total) + int(n))%})
%pyset(total, %add(10))
%pyset(total, %add(20))
%pyset(total, %add(12))
%pyget(total)"#;
        let result = eval_string(src, None, &mut ev).expect("eval failed");
        assert_eq!(result.trim(), "42");
    }
}
