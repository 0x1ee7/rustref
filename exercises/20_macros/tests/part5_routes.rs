//! Part 5: `routes!`, a declarative route table validated at construction.

use std::fmt::Debug;

use ex20_macros::{routes, Lookup, Match, Method, Route, RouteError, Router};

fn users_table() -> Result<Router<&'static str>, RouteError> {
    routes! {
        GET "/" => "root",
        GET "/users" => "list_users",
        POST "/users" => "create_user",
        GET "/users/me" => "me",
        GET "/users/:id" => "get_user",
        PUT "/users/:id" => "replace_user",
        DELETE "/users/:id" => "delete_user",
        GET "/users/:id/posts/:post" => "get_post",
        PATCH "/users/:id/posts/:post" => "patch_post",
    }
}

fn users() -> Router<&'static str> {
    users_table().expect("the users table is valid")
}

fn found<'r, 'p, H: Debug>(lookup: Lookup<'r, 'p, H>) -> Match<'r, 'p, H> {
    match lookup {
        Lookup::Found(m) => m,
        other => panic!("expected Lookup::Found, got {other:?}"),
    }
}

fn err<H>(result: Result<Router<H>, RouteError>) -> RouteError {
    match result {
        Ok(_) => panic!("expected the route table to be rejected"),
        Err(e) => e,
    }
}

#[test]
fn part5_table_builds_every_route() {
    let router = users();
    assert_eq!(router.len(), 9, "all nine routes should be kept");
    assert!(!router.is_empty());
}

#[test]
fn part5_empty_table_is_valid() {
    fn empty() -> Result<Router<u8>, RouteError> {
        routes! {}
    }
    let router = empty().expect("an empty table is valid");
    assert!(router.is_empty());
    assert_eq!(router.len(), 0);
}

#[test]
fn part5_lookup_static_and_root_routes() {
    let router = users();
    assert_eq!(*found(router.lookup(Method::Get, "/users")).handler, "list_users");
    assert_eq!(*found(router.lookup(Method::Get, "/")).handler, "root");
    assert_eq!(*found(router.lookup(Method::Post, "/users")).handler, "create_user");
}

#[test]
fn part5_lookup_captures_params_in_pattern_order() {
    let router = users();
    let m = found(router.lookup(Method::Get, "/users/7/posts/9"));
    assert_eq!(*m.handler, "get_post");
    assert_eq!(
        m.params,
        vec![("id", "7"), ("post", "9")],
        "params should follow pattern order"
    );
    assert_eq!(m.param("post"), Some("9"));
    assert_eq!(m.param("nope"), None, "unknown names yield None");
}

#[test]
fn part5_literal_beats_param_and_backtracks() {
    let router = users();

    let exact = found(router.lookup(Method::Get, "/users/me"));
    assert_eq!(*exact.handler, "me", "a literal segment beats :id");
    assert!(exact.params.is_empty());

    let fallback = found(router.lookup(Method::Get, "/users/me/posts/3"));
    assert_eq!(
        *fallback.handler, "get_post",
        "when the literal branch cannot complete, fall back to :id"
    );
    assert_eq!(fallback.param("id"), Some("me"));
    assert_eq!(fallback.param("post"), Some("3"));
}

#[test]
fn part5_lookup_method_not_allowed_lists_sorted_methods() {
    let router = users();
    match router.lookup(Method::Post, "/users/42") {
        Lookup::MethodNotAllowed(methods) => assert_eq!(
            methods,
            vec![Method::Get, Method::Put, Method::Delete],
            "allowed methods should be sorted and de-duplicated"
        ),
        other => panic!("expected MethodNotAllowed, got {other:?}"),
    }
}

#[test]
fn part5_lookup_not_found_cases() {
    let router = users();
    assert!(
        matches!(router.lookup(Method::Get, "/nope"), Lookup::NotFound),
        "an unknown path is NotFound"
    );
    assert!(
        matches!(router.lookup(Method::Get, "users"), Lookup::NotFound),
        "a path without a leading slash is NotFound"
    );
    assert!(
        matches!(router.lookup(Method::Get, "/users/"), Lookup::NotFound),
        "there is no trailing-slash normalisation, and :id needs a non-empty segment"
    );
}

#[test]
fn part5_route_new_and_method_display() {
    let route = Route::new(Method::Patch, "/x", 5u8);
    assert_eq!(route.method, Method::Patch);
    assert_eq!(route.pattern, "/x");
    assert_eq!(route.handler, 5);
    assert_eq!(Method::Patch.as_str(), "PATCH");
    assert_eq!(Method::Delete.to_string(), "DELETE");
}

#[test]
fn part5_duplicate_shape_is_rejected() {
    let e = err::<u8>(routes! {
        GET "/users/:id" => 1u8,
        GET "/users/:name" => 2u8,
    });
    assert_eq!(
        e,
        RouteError::Duplicate {
            method: Method::Get,
            pattern: "/users/:name",
            existing: "/users/:id",
        },
        "parameter names do not change a route's shape"
    );
}

#[test]
fn part5_invalid_leading_slash_and_empty_segments_are_rejected() {
    let missing = err::<u8>(routes! { GET "users" => 1u8 });
    assert!(
        matches!(missing, RouteError::InvalidPattern { pattern: "users", .. }),
        "got {missing:?}"
    );

    let double = err::<u8>(routes! { GET "/users//x" => 1u8 });
    assert!(
        matches!(double, RouteError::InvalidPattern { pattern: "/users//x", .. }),
        "got {double:?}"
    );

    let trailing = err::<u8>(routes! { GET "/users/" => 1u8 });
    assert!(
        matches!(trailing, RouteError::InvalidPattern { pattern: "/users/", .. }),
        "got {trailing:?}"
    );
}

#[test]
fn part5_invalid_param_names_are_rejected() {
    let empty_name = err::<u8>(routes! { GET "/users/:" => 1u8 });
    assert!(
        matches!(empty_name, RouteError::InvalidPattern { pattern: "/users/:", .. }),
        "a bare ':' has no parameter name, got {empty_name:?}"
    );

    let repeated = err::<u8>(routes! { GET "/a/:id/b/:id" => 1u8 });
    assert!(
        matches!(repeated, RouteError::InvalidPattern { pattern: "/a/:id/b/:id", .. }),
        "duplicate parameter names are rejected, got {repeated:?}"
    );
}

#[test]
fn part5_first_offending_route_is_reported() {
    let e = err::<u8>(routes! {
        GET "/ok" => 1u8,
        GET "bad" => 2u8,
        GET "/ok" => 3u8,
    });
    assert!(
        matches!(e, RouteError::InvalidPattern { pattern: "bad", .. }),
        "validation runs in table order, got {e:?}"
    );
}

#[test]
fn part5_error_display_mentions_pattern() {
    let e = err::<u8>(routes! {
        GET "/users/:id" => 1u8,
        GET "/users/:name" => 2u8,
    });
    assert!(
        e.to_string().contains("/users/:name"),
        "the Display text should name the offending pattern, got {e}"
    );
}
