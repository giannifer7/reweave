use crate::evaluator::{EvalConfig, EvalError, Evaluator};
use crate::macro_api::{process_string, process_string_defaults};

#[test]
fn test_def_macro_other_errors() {
    // Test missing arguments
    let result = process_string_defaults("%def()");
    assert!(
        matches!(result, Err(EvalError::InvalidUsage(_))),
        "Expected InvalidUsage error for empty def"
    );

    // Test single argument
    let result = process_string_defaults("%def(foo)");
    assert!(
        matches!(result, Err(EvalError::InvalidUsage(_))),
        "Expected InvalidUsage error for def with only name"
    );

    // Test numeric name
    let result = process_string_defaults("%def(123, body)");
    assert!(
        matches!(result, Err(EvalError::InvalidUsage(_))),
        "Expected InvalidUsage error for numeric macro name"
    );

    // Test numeric parameter
    let result = process_string_defaults("%def(foo, 123, body)");
    assert!(
        matches!(result, Err(EvalError::InvalidUsage(_))),
        "Expected InvalidUsage error for numeric parameter"
    );

    // Test parameter with equals
    let result = process_string_defaults("%def(foo, param=value, body)");
    assert!(
        matches!(result, Err(EvalError::InvalidUsage(_))),
        "Expected InvalidUsage error for parameter with equals"
    );
}

#[test]
fn test_def_macro_basic() {
    let result = process_string_defaults("%def(foo, bar) [%foo()]").unwrap();
    assert_eq!(result, b" [bar]");
}

#[test]
fn test_def_macro_with_params() {
    let result = process_string_defaults(
        "%def(greet, name, message, %{Hello, %(name)! %(message)%})\n%greet(Alice, Have a nice day)",
    )
    .unwrap();
    assert_eq!(
        std::str::from_utf8(&result).unwrap(),
        "\nHello, Alice! Have a nice day"
    );
}

#[test]
fn test_def_macro_trailing_comma_is_ignored() {
    let result = process_string_defaults("%def(foo, bar,)\n%foo()").unwrap();
    assert_eq!(std::str::from_utf8(&result).unwrap(), "\nbar");
}

#[test]
fn test_def_macro_comma_errors() {
    let result = process_string_defaults("%def(foo bar baz, body)");
    assert!(matches!(result, Err(EvalError::InvalidUsage(_))));

    let result = process_string_defaults("%def(,)");
    assert!(matches!(result, Err(EvalError::InvalidUsage(_))));

    let result = process_string_defaults("%def(foo,,)");
    assert!(matches!(result, Err(EvalError::InvalidUsage(_))));
}

#[test]
fn test_def_macro_with_comments() {
    let result = process_string_defaults(
        "%def(greet, %/* greeting macro %*/\n\
         name, %// person to greet\n\
         msg, %/* message to show %*/\n\
         Hello %(name)! %(msg)\n\
         )\n\
         %greet(Alice, Good morning)",
    )
    .unwrap();
    assert_eq!(
        std::str::from_utf8(&result).unwrap(),
        "\nHello Alice! Good morning\n"
    );
}

#[test]
fn test_def_macro_spaces() {
    let result = process_string_defaults("%def( foo, bar, baz, output)\n%foo(bar, baz)").unwrap();
    assert_eq!(std::str::from_utf8(&result).unwrap(), "\noutput");
}

#[test]
fn test_def_macro_nested() {
    let result = process_string_defaults(
        "%def(bold, text, **%(text)**)
         %def(greet, name, Hello %bold(dear %(name))!)
         %greet(World)",
    )
    .unwrap();
    assert_eq!(
        std::str::from_utf8(&result).unwrap(),
        "\n         \n         Hello **dear World**!"
    );
}

#[test]
fn test_recursion_depth_limit_returns_error() {
    // A directly self-recursive macro must hit MAX_RECURSION_DEPTH and
    // return a Runtime error rather than stack-overflowing the process.
    let result = process_string_defaults("%def(loop, %loop())\n%loop()");
    assert!(
        matches!(result, Err(EvalError::Runtime(_))),
        "expected Runtime error for infinite recursion, got {:?}",
        result
    );
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("recursion") || msg.contains("depth"),
        "error message should mention recursion depth: {}",
        msg
    );
}

#[test]
fn test_mutual_recursion_depth_limit() {
    // Mutually recursive macros: %a calls %b, %b calls %a.
    let src = "%def(a, %b())\n%def(b, %a())\n%a()";
    let result = process_string_defaults(src);
    assert!(
        matches!(result, Err(EvalError::Runtime(_))),
        "expected Runtime error for mutual recursion, got {:?}",
        result
    );
}

#[test]
fn test_def_rejects_same_frame_redefinition() {
    let result = process_string_defaults("%def(compute, x, %(x))\n%def(compute, x, %(x))");
    let err = result.unwrap_err().to_string();
    assert!(err.contains("constant binding"));
}

#[test]
fn test_def_rejects_rebinding_rebindable_name() {
    let result = process_string_defaults("%redef(compute, x, %(x))\n%def(compute, x, %(x))");
    let err = result.unwrap_err().to_string();
    assert!(err.contains("rebindable binding"));
    assert!(err.contains("%redef"));
}

#[test]
fn test_redef_creates_and_replaces_rebindable_macro() {
    let src = "%redef(compute, x, %(x))\n%redef(compute, x, %(x)!)\n%compute(hello)";
    let result = process_string_defaults(src).unwrap();
    let output = String::from_utf8(result).unwrap();
    assert_eq!(output.trim(), "hello!");
}

#[test]
fn test_redef_rejects_constant_name() {
    let result = process_string_defaults("%def(compute, x, %(x))\n%redef(compute, x, %(x)!)");
    let err = result.unwrap_err().to_string();
    assert!(err.contains("constant binding"));
}

#[test]
fn test_param_with_hyphen_is_rejected() {
    // Hyphens lex as Special tokens; `single_ident_param` must reject them.
    let result = process_string_defaults("%def(foo, my-param, body)");
    assert!(
        matches!(result, Err(EvalError::InvalidUsage(_))),
        "expected InvalidUsage for hyphenated param name, got {:?}",
        result
    );
}

#[test]
fn test_eager_argument_evaluation_order() {
    // Arguments are evaluated in CALLER scope, before the callee frame is pushed.
    // %set in argument position is forbidden, so use a pure caller-scope read.
    let src = "%set(counter, 1)\n\
         %def(show, x, %{arg=%(x)%})\n\
         %show(%(counter))";
    let mut ev = Evaluator::new(EvalConfig::default());
    let result = process_string(src, None, &mut ev).unwrap();
    let output = String::from_utf8(result).unwrap();
    assert!(
        output.contains("arg=1"),
        "expected caller-scope argument evaluation, got: {:?}",
        output
    );
}

#[test]
fn test_arguments_evaluated_before_body() {
    // Verify strictness: argument expressions are fully expanded to strings
    // before the macro body executes.
    let src = "%def(loud, x, %(x)!)\n\
         %def(join, a, b, %(a)%(b))\n\
         %join(%loud(hi), %loud(there))";
    let result = process_string_defaults(src).unwrap();
    let output = String::from_utf8(result).unwrap();
    assert_eq!(
        output.trim(),
        "hi!there!",
        "expected eager evaluation: hi! and there! expanded before join body runs"
    );
}
