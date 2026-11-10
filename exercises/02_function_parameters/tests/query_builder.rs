//! Part 5: a consuming typestate builder. `build` exists only after a table is
//! chosen. Identifier and limit checks happen in `build`.

use ex02_function_parameters::{QueryError, Value, select};

#[test]
fn part5_empty_columns_render_star_and_columns_render_list() {
    assert_eq!(
        select(Vec::<&str>::new()).from_table("users").build(),
        Ok(("SELECT * FROM users".to_string(), vec![]))
    );
    assert_eq!(
        select(["id", "u.name"]).from_table("public.users").build(),
        Ok(("SELECT id, u.name FROM public.users".to_string(), vec![]))
    );
}

#[test]
fn part5_invalid_column_identifier_is_rejected() {
    assert_eq!(
        select(["id; DROP TABLE x"]).from_table("t").build(),
        Err(QueryError::InvalidIdentifier("id; DROP TABLE x".to_string()))
    );
}

#[test]
fn part5_invalid_table_and_filter_identifiers_are_rejected() {
    assert_eq!(
        select(["id"]).from_table("1users").build(),
        Err(QueryError::InvalidIdentifier("1users".to_string())),
        "identifiers cannot start with a digit"
    );
    assert_eq!(
        select(["id"])
            .from_table("users")
            .filter_eq("a b", 1i64)
            .build(),
        Err(QueryError::InvalidIdentifier("a b".to_string()))
    );
}

#[test]
fn part5_filter_renders_placeholder_or_is_null() {
    assert_eq!(
        select(["id"])
            .from_table("users")
            .filter_eq("age", 30i64)
            .build(),
        Ok((
            "SELECT id FROM users WHERE age = ?".to_string(),
            vec![Value::Int(30)]
        ))
    );
    assert_eq!(
        select(["id"])
            .from_table("users")
            .filter_eq("deleted_at", None::<i64>)
            .build(),
        Ok(("SELECT id FROM users WHERE deleted_at IS NULL".to_string(), vec![])),
        "a null value renders IS NULL with no parameter"
    );
}

#[test]
fn part5_filters_join_with_and_in_call_order() {
    assert_eq!(
        select(Vec::<&str>::new())
            .from_table("t")
            .filter_eq("a", true)
            .filter_eq("b", "x")
            .build(),
        Ok((
            "SELECT * FROM t WHERE a = ? AND b = ?".to_string(),
            vec![Value::Bool(true), Value::Text("x".to_string())]
        ))
    );
}

#[test]
fn part5_order_terms_respect_leading_minus_for_desc() {
    assert_eq!(
        select(Vec::<&str>::new())
            .from_table("t")
            .order_by("name")
            .order_by("-created_at")
            .build(),
        Ok((
            "SELECT * FROM t ORDER BY name, created_at DESC".to_string(),
            vec![]
        ))
    );
}

#[test]
fn part5_invalid_order_terms_are_rejected() {
    assert_eq!(
        select(Vec::<&str>::new())
            .from_table("t")
            .order_by("name asc")
            .build(),
        Err(QueryError::InvalidOrderTerm("name asc".to_string()))
    );
    assert_eq!(
        select(Vec::<&str>::new()).from_table("t").order_by("-").build(),
        Err(QueryError::InvalidOrderTerm("-".to_string())),
        "a bare minus has no identifier"
    );
}

#[test]
fn part5_limit_is_a_bound_parameter_and_last_call_wins() {
    assert_eq!(
        select(Vec::<&str>::new()).from_table("t").limit(10).build(),
        Ok(("SELECT * FROM t LIMIT ?".to_string(), vec![Value::Int(10)]))
    );
    assert_eq!(
        select(Vec::<&str>::new())
            .from_table("t")
            .limit(5)
            .limit(7)
            .build(),
        Ok(("SELECT * FROM t LIMIT ?".to_string(), vec![Value::Int(7)]))
    );
}

#[test]
fn part5_limit_beyond_i64_is_rejected() {
    assert_eq!(
        select(Vec::<&str>::new())
            .from_table("t")
            .limit(u64::MAX)
            .build(),
        Err(QueryError::LimitTooLarge(u64::MAX))
    );
    assert!(
        select(Vec::<&str>::new())
            .from_table("t")
            .limit(i64::MAX as u64)
            .build()
            .is_ok(),
        "i64::MAX itself fits the bound parameter"
    );
}

#[test]
fn part5_full_query_renders_placeholders_in_order() {
    assert_eq!(
        select(["id"])
            .from_table("t")
            .filter_eq("a", 1i64)
            .order_by("-id")
            .limit(3)
            .build(),
        Ok((
            "SELECT id FROM t WHERE a = ? ORDER BY id DESC LIMIT ?".to_string(),
            vec![Value::Int(1), Value::Int(3)]
        ))
    );
}

#[test]
fn part5_value_from_conversions_cover_options_and_strings() {
    assert_eq!(Value::from(true), Value::Bool(true));
    assert_eq!(Value::from(7i64), Value::Int(7));
    assert_eq!(Value::from("x"), Value::Text("x".to_string()));
    assert_eq!(Value::from(String::from("s")), Value::Text("s".to_string()));
    assert_eq!(Value::from(Some("x")), Value::Text("x".to_string()));
    assert_eq!(Value::from(None::<bool>), Value::Null);
}
